//! Provides a walker capable of iterating over directories and providing relevant information
//! for reading, processing, and writing SVG documents.
use std::{
    ffi::OsStr,
    io::{IsTerminal, Read},
    path::PathBuf,
};

use anyhow::anyhow;
use ignore::{WalkBuilder, WalkState};
use oxvg_ast::{node::Ref, serialize::Node as _, xmlwriter::Options};

type FnVisitor = Box<dyn FnMut(&str, Option<&PathBuf>, Option<&PathBuf>) + Send>;

/// This will iterate over a set of paths.
pub struct Walk<'a> {
    /// The set of paths to visit
    pub paths: &'a [PathBuf],
    /// Writes to the given paths instead of the input path when specified.
    pub output: Option<&'a PathBuf>,
    /// If the path is a directory, whether to walk through and optimise its
    /// subdirectories
    pub recursive: bool,
    /// Whether to search through hidden files and directories
    pub hidden: bool,
    /// Whether to disregard ignore patterns
    pub no_ignore: bool,
    /// Sets the approximate number of threads to use. A value of 0 will
    /// automatically determine the appropriate number
    pub threads: usize,
}

pub(crate) struct Output<'a, 'input, 'arena> {
    pub options: Options,
    pub dom: Ref<'input, 'arena>,
    pub input: Option<&'a PathBuf>,
    pub destination: Option<&'a PathBuf>,
    pub input_size: f64,
}
impl Output<'_, '_, '_> {
    pub fn output(self) -> anyhow::Result<()> {
        let is_stdin = self.input.is_none();
        let input_size = self.input_size;
        if let Some(output) = self.destination {
            if is_stdin && output.metadata().is_ok_and(|f| f.is_dir()) {
                eprintln!("Cannot use dir as output with stdin. Printing result to stdout instead");
                self.dom.serialize_into(std::io::stdout(), self.options)?;
            } else {
                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let file = std::fs::File::create(output)?;
                self.dom.serialize_into(file, self.options)?;

                let output_size = output.metadata()?.len() as f64 / 1000.0;
                let change = 100.0 * (input_size - output_size) / input_size;
                let increased = if change < 0.0 { "\x1b[31m" } else { "" };
                let path = self.input.and_then(|p| p.to_str()).unwrap_or("");
                println!(
            "\n\n\x1b[32m{path:?} ({input_size:.1}KB) -> {output:?} ({output_size:.1}KB) {increased}({change:.2}%)\x1b[0m"
                );
            }
        } else {
            self.dom.serialize_into(std::io::stdout(), self.options)?;
        }
        Ok(())
    }
}

impl Walk<'_> {
    /// Start visiting the paths in parallel. `f` is called for each thread
    /// and the resulting function is called for each path.
    ///
    /// # Errors
    ///
    /// When invalid options are passed to [`Walk`].
    pub fn run<F: Fn() -> FnVisitor>(&self, f: F) -> anyhow::Result<()> {
        if !std::io::stdin().is_terminal()
            && self.paths.len() <= 1
            && self
                .paths
                .first()
                .is_none_or(|path| path.as_os_str() == OsStr::new("."))
        {
            return self.handle_stdin(f());
        }
        if self.paths.is_empty() {
            return Err(anyhow!(
                "This command requires at least one path to optimise"
            ));
        }

        for path in self.paths {
            self.handle_path(path, &f);
        }
        Ok(())
    }

    fn handle_stdin(&self, mut f: FnVisitor) -> anyhow::Result<()> {
        let mut source = String::new();
        std::io::stdin().read_to_string(&mut source)?;
        f(&source, None, self.output);
        Ok(())
    }

    fn handle_path<F: Fn() -> FnVisitor>(&self, path: &PathBuf, f: F) {
        let output_path = |input: &PathBuf| {
            let Some(output) = self.output else {
                return Ok(None);
            };
            input.strip_prefix(path).map(|p| {
                Some(if p.as_os_str().is_empty() {
                    output.clone()
                } else {
                    output.join(p)
                })
            })
        };
        WalkBuilder::new(path)
            .max_depth(if self.recursive { None } else { Some(1) })
            .hidden(!self.hidden)
            .git_ignore(!self.no_ignore)
            .ignore(!self.no_ignore)
            .follow_links(true)
            .threads(self.threads)
            .build_parallel()
            .run(|| {
                let mut visitor = f();
                Box::new(move |path| {
                    let Ok(path) = path else {
                        return WalkState::Continue;
                    };
                    if path.file_type().is_none_or(|f| !f.is_file()) {
                        return WalkState::Continue;
                    }
                    let path = path.into_path();
                    if path.extension().and_then(OsStr::to_str) != Some("svg") {
                        return WalkState::Continue;
                    }
                    let Ok(output_path) = output_path(&path) else {
                        return WalkState::Continue;
                    };
                    let Ok(file) = std::fs::read_to_string(path.clone()) else {
                        return WalkState::Continue;
                    };
                    visitor(&file, Some(&path), output_path.as_ref());
                    WalkState::Continue
                })
            });
    }
}
