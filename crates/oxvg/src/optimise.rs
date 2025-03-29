use std::{
    cell::RefCell,
    ffi::OsStr,
    io::{IsTerminal, Read, Write},
    path::PathBuf,
    str::FromStr,
};

use anyhow::anyhow;
use ignore::{WalkBuilder, WalkState};
use oxvg_ast::{
    implementations::markup5ever::{Element5Ever, Node5Ever},
    visitor::Info,
};
use oxvg_optimiser::Jobs;

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
}

impl RunCommand for Optimise {
    fn run(&self, config: Config) -> anyhow::Result<()> {
        let config = self.handle_config(config)?;
        let Some(config) = config else {
            return Ok(());
        };
        if let Some(jobs) = config.optimise {
            LOADED_JOBS.set(jobs.resolve_jobs());
        }

        self.handle_paths()
    }
}

impl Optimise {
    fn handle_out<W: Write>(dom: &Node5Ever, wr: W) -> anyhow::Result<()> {
        use oxvg_ast::serialize::Node;

        dom.serialize_into(wr)
    }

    fn handle_stdin(&self, jobs: Jobs<Element5Ever>) -> anyhow::Result<()> {
        use oxvg_ast::parse::Node;

        let mut string = String::new();
        std::io::stdin().read_to_string(&mut string)?;
        let dom = Node5Ever::parse(&string)?;

        let info = Info {
            path: None,
            multipass_count: 0,
        };
        jobs.run(&dom, &info)?;

        if let Some(output) = &self.output.as_ref().and_then(|o| {
            eprintln!("Warning: Using empty `-o,--output` with stdin will print to stdout, you can instead omit `-o,--output`.");
            o.first()
        }) {
            let file = std::fs::File::open(output)?;
            if file.metadata()?.is_file() {
                eprintln!(
                    "Cannot use dir as output for stdin input. Printing result to stdout instead"
                );
                Self::handle_out(&dom, std::io::stdout())?;
            } else {
                Self::handle_out(&dom, file)?;
            }
        } else {
            Self::handle_out(&dom, std::io::stdout())?;
        }

        Ok(())
    }

    fn handle_file(
        jobs: &Jobs<Element5Ever>,
        path: &PathBuf,
        output: Option<&PathBuf>,
    ) -> anyhow::Result<()> {
        use oxvg_ast::parse::Node;

        let file = std::fs::File::open(path)?;
        let input_size = file.metadata()?.len() as f64 / 1000.0;
        let dom = Node5Ever::parse_file(&file)?;
        drop(file);

        let info = Info {
            path: Some(path.clone()),
            multipass_count: 0,
        };
        jobs.clone().run(&dom, &info)?;

        if let Some(output_path) = output {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = std::fs::File::create(output_path)?;
            Self::handle_out(&dom, file)?;

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
            Self::handle_out(&dom, std::io::stdout())
        }
    }

    fn handle_path(&self, path: &PathBuf) {
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
                    let jobs = LOADED_JOBS.with_borrow(Clone::clone);
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
                    if let Err(err) = Self::handle_file(&jobs, &path, output_path.as_ref()) {
                        eprintln!("{err}");
                    };
                    WalkState::Continue
                })
            });
    }

    fn handle_paths(&self) -> anyhow::Result<()> {
        if !std::io::stdin().is_terminal()
            && self.paths.len() <= 1
            && self
                .paths
                .first()
                .is_none_or(|path| path == &PathBuf::from_str(".").unwrap())
        {
            return LOADED_JOBS.with(|jobs| self.handle_stdin(jobs.take()));
        }
        if self.paths.is_empty() {
            return Err(anyhow!(
                "`oxvg optimise` requires at least one path to optimise"
            ));
        }

        for path in &self.paths {
            self.handle_path(path);
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
        } else {
            log::debug!("using inferred config");
            Ok(Some(config))
        }
    }
}

thread_local! {
    static LOADED_JOBS: RefCell<Jobs<Element5Ever>> = RefCell::new(Jobs::default());
}
