use std::path::PathBuf;

use clap::Subcommand;
use oxvg_lint::{error::LintingError, Rules, Severity};

use crate::{args::RunCommand, config::Config, walk::Walk};

mod lsp;

#[derive(clap::Args, Debug)]
pub struct Check {
    #[clap(flatten)]
    pub walk: Walk,
    /// A path to the specified config.
    #[clap(long, short, num_args(0..=1))]
    pub config: Option<Vec<PathBuf>>,
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
    fn walk(mut self, rules: &Rules) -> anyhow::Result<()> {
        self.walk.output = None;
        self.walk.run(|| {
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
    ///
    /// Note: The `-output` flag has no effect, results will always be printed to stdout.
    Check(Check),
    /// Start a server for an editor or other type of client to analyse documents
    Serve(Serve),
}
