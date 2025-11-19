use std::{
    ffi::OsStr,
    io::{IsTerminal, Read, Write},
    path::PathBuf,
    str::FromStr,
};

use anyhow::anyhow;
use ignore::{WalkBuilder, WalkState};
use oxvg_ast::{
    arena::Allocator,
    node::Ref,
    parse::roxmltree::parse,
    serialize::Node as _,
    visitor::Info,
    xmlwriter::{Indent, Options},
};
use oxvg_optimiser::{Extends, Jobs};
use roxmltree::ParsingOptions;

use crate::{args::RunCommand, config::Config};

#[derive(clap::Args, Debug)]
pub struct Optimise {
    /// The target paths to optimise
    #[clap(value_parser)]
    pub paths: Vec<PathBuf>,
    /// Whether to write to the specified file or directory.
    /// Will use the input if flag is given without a value.
    /// Defaults to stdout.
    #[clap(long, short, num_args(0..=1))]
    pub output: Option<Vec<PathBuf>>,
    /// A path to the specified config.
    /// If no config is specified the current config will be printed instead.
    #[clap(long, short, num_args(0..=1))]
    pub config: Option<Vec<PathBuf>>,
    /// If the path is a directory, whether to walk through and optimise it's subdirectories
    #[clap(long, short, default_value = "false")]
    pub recursive: bool,
    /// Search through hidden files and directories
    #[clap(long, short = '.', default_value = "false")]
    pub hidden: bool,
    /// Sets the approximate number of threads to use. A value of 0 (default) will automatically determine the appropriate number
    #[clap(long, short, default_value = "0")]
    pub threads: usize,
    /// When running without a config, sets the default preset to run with
    #[clap(long, short)]
    pub extends: Option<Extends>,
}

impl RunCommand for Optimise {
    fn run(&self, config: Config) -> anyhow::Result<()> {
        let config = self.handle_config(config)?;
        let Some(config) = config else {
            return Ok(());
        };

        self.handle_paths(
            &config
                .optimise
                .map(|j| j.resolve_jobs())
                .unwrap_or_default(),
        )
    }
}

impl Optimise {
    fn handle_out<W: Write>(dom: Ref, wr: W) -> anyhow::Result<W> {
        Ok(dom.serialize_into(
            wr,
            Options {
                indent: Indent::None,
                ..Options::default()
            },
        )?)
    }

    fn handle_stdin(&self, jobs: &Jobs) -> anyhow::Result<()> {
        let mut source = String::new();
        std::io::stdin().read_to_string(&mut source)?;
        let xml = roxmltree::Document::parse_with_options(
            &source,
            ParsingOptions {
                allow_dtd: true,
                ..ParsingOptions::default()
            },
        )
        .unwrap();
        let values = Allocator::new_values();
        let mut arena = Allocator::new_arena();
        let mut allocator = Allocator::new(&mut arena, &values);
        let dom = parse(&xml, &mut allocator)?;

        let info = Info {
            path: None,
            multipass_count: 0,
            allocator,
        };
        jobs.run(dom, &info)
            .map_err(|e| anyhow::Error::msg(e.to_string()))?;

        if let Some(output) = &self.output.as_ref().and_then(|o| {
            eprintln!("Warning: Using empty `-o,--output` with stdin will print to stdout, you can instead omit `-o,--output`.");
            o.first()
        }) {
            let file = std::fs::File::open(output)?;
            if file.metadata()?.is_file() {
                eprintln!(
                    "Cannot use dir as output for stdin input. Printing result to stdout instead"
                );
                Self::handle_out(dom, std::io::stdout())?;
            } else {
                Self::handle_out(dom, file)?;
            }
        } else {
            Self::handle_out(dom, std::io::stdout())?;
        }

        Ok(())
    }

    fn handle_file(jobs: &Jobs, path: &PathBuf, output: Option<&PathBuf>) -> anyhow::Result<()> {
        let file = std::fs::read_to_string(path)?;
        let input_size = file.len() as f64 / 1000.0;
        let xml = roxmltree::Document::parse_with_options(
            &file,
            ParsingOptions {
                allow_dtd: true,
                ..ParsingOptions::default()
            },
        )
        .unwrap();
        let values = Allocator::new_values();
        let mut arena = Allocator::new_arena();
        let mut allocator = Allocator::new(&mut arena, &values);
        let dom = parse(&xml, &mut allocator).unwrap();

        let info: Info = Info {
            path: Some(path.clone()),
            multipass_count: 0,
            allocator,
        };
        jobs.run(dom, &info)
            .map_err(|e| anyhow::Error::msg(e.to_string()))?;

        if let Some(output_path) = output {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = std::fs::File::create(output_path)?;
            Self::handle_out(dom, file)?;

            let output_size = output_path.metadata()?.len() as f64 / 1000.0;
            let change = 100.0 * (input_size - output_size) / input_size;
            let increased = if change < 0.0 { "\x1b[31m" } else { "" };
            println!(
                "\n\n\x1b[32m{path:?} ({input_size:.1}KB) -> {output_path:?} ({output_size:.1}KB) {increased}({change:.2}%)\x1b[0m"
            );
            Ok(())
        } else {
            // Print to stderr, so that stdout is clean for writing
            eprintln!("\n\n\x1b[32m{}\x1b[0m", path.to_string_lossy());
            Self::handle_out(dom, std::io::stdout()).map(|_| ())
        }
    }

    fn handle_path(&self, path: &PathBuf, jobs: &Jobs) {
        let output_path = |input: &PathBuf| {
            let Some(output) = self.output.as_ref() else {
                return Ok(None);
            };
            let Some(output) = output.first() else {
                return Ok(Some(input.clone()));
            };
            input.strip_prefix(path).map(|p| Some(output.join(p)))
        };
        WalkBuilder::new(path)
            .max_depth(if self.recursive { None } else { Some(1) })
            .hidden(!self.hidden)
            .git_ignore(!self.hidden)
            .ignore(!self.hidden)
            .follow_links(true)
            .threads(self.threads)
            .build_parallel()
            .run(|| {
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
                    if let Err(err) = Self::handle_file(jobs, &path, output_path.as_ref()) {
                        eprintln!(
                            "{}: \x1b[31m{err}\x1b[0m",
                            path.to_str().unwrap_or_default()
                        );
                    }
                    WalkState::Continue
                })
            });
    }

    fn handle_paths(&self, jobs: &Jobs) -> anyhow::Result<()> {
        if !std::io::stdin().is_terminal()
            && self.paths.len() <= 1
            && self
                .paths
                .first()
                .is_none_or(|path| path == &PathBuf::from_str(".").unwrap())
        {
            return self.handle_stdin(jobs);
        }
        if self.paths.is_empty() {
            return Err(anyhow!(
                "`oxvg optimise` requires at least one path to optimise"
            ));
        }

        for path in &self.paths {
            self.handle_path(path, jobs);
        }
        Ok(())
    }

    fn handle_config(&self, config: Config) -> anyhow::Result<Option<Config>> {
        if let Some(config_paths) = &self.config {
            if let Some(config_path) = config_paths.first() {
                log::debug!("using specified config");
                let file = std::fs::File::open(config_path)?;
                serde_json::from_reader(file)
                    .map_err(|e| anyhow!(e))
                    .map(Some)
            } else {
                log::debug!("printing config");
                serde_json::to_writer(
                    std::io::stdout(),
                    &Config {
                        optimise: Some(config.optimise.unwrap_or_default()),
                    },
                )?;
                Ok(None)
            }
        } else if let Some(extends) = &self.extends {
            Ok(Some(Config {
                optimise: Some(crate::config::Optimise {
                    extends: Some(extends.clone()),
                    jobs: Jobs::none(),
                    omit: None,
                }),
            }))
        } else {
            log::debug!("using inferred config");
            Ok(Some(config))
        }
    }
}
