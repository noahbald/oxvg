use std::path::PathBuf;

use clap::Subcommand;
use oxvg_lint::{error::LintingError, Rules, Severity};

use crate::{args::RunCommand, config::Config, walk::Walk};

mod lsp;

#[derive(clap::Args, Debug)]
pub struct Check {
    /// The target paths to optimise
    #[clap(value_parser)]
    pub paths: Vec<PathBuf>,
    /// A path to the specified config.
    #[clap(long, short, num_args(0..=1))]
    pub config: Option<Vec<PathBuf>>,
    /// If the path is a directory, whether to walk through and optimise its subdirectories
    #[clap(long, short, default_value = "false")]
    pub recursive: bool,
    /// Search through hidden files and directories
    #[clap(long, short = '.', default_value = "false")]
    pub hidden: bool,
    /// Sets the approximate number of threads to use. A value of 0 (default) will automatically determine the appropriate number
    #[clap(long, short, default_value = "0")]
    pub threads: usize,
    #[clap(long, short, default_value = "error")]
    /// Sets the level at which the program will exit with an error code.
    pub level: Severity,
}
impl RunCommand for Check {
    async fn run(self, config: Config) -> anyhow::Result<()> {
        self.walk(&config.lint.unwrap_or_else(Rules::recommended))
    }
}
impl Check {
    fn walk(self, rules: &Rules) -> anyhow::Result<()> {
        let walk = Walk {
            paths: &self.paths,
            output: None,
            recursive: self.recursive,
            hidden: self.hidden,
            threads: self.threads,
        };
        walk.run(|| {
            let rules = rules.clone();
            Box::new(move |source, path, _| {
                let result = rules.lint_with_path(source, path);
                match result {
                    Err(LintingError::Reported { .. }) => {}
                    Err(err) => eprintln!("{err}"),
                    _ => {}
                }
            })
        })
    }
}

#[derive(clap::Args, Debug)]
pub struct Serve {
    /// A path to the specified config.
    #[clap(long, short, num_args(0..=1))]
    pub config: Option<Vec<PathBuf>>,
}
impl RunCommand for Serve {
    async fn run(self, config: Config) -> anyhow::Result<()> {
        lsp::serve(config.lint.unwrap_or_else(Rules::recommended)).await;
        Ok(())
    }
}

#[derive(Subcommand)]
/// Run analysis against input files and reports any problems that it may detect.
pub enum Lint {
    /// Analyse SVG documents for problems and report them to standard output
    Check(Check),
    /// Start a server for an editor or other type of client to analyse documents
    Serve(Serve),
}
