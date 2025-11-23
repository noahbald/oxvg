use std::path::PathBuf;

use oxvg_ast::xmlwriter::{Indent, Space};
use oxvg_optimiser::{Extends, Jobs};

use crate::{args::RunCommand, commands::Optimise, config::Config};

#[derive(clap::Args, Debug)]
/// Runs [`Optimise`] with options and defaults specialised for formatting
pub struct Format {
    /// The target paths to optimise
    #[clap(value_parser)]
    pub paths: Vec<PathBuf>,
    /// Whether to write to the specified file or directory.
    /// Will use the input if flag is given without a value.
    /// Defaults to stdout.
    #[clap(long, short, num_args(0..=1))]
    pub output: Option<Vec<PathBuf>>,
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
    #[clap(long, short, default_value = "4")]
    pub pretty: Indent,
    /// Controls how the output handles whitespace.
    #[clap(long, short, default_value = "auto")]
    pub space: Space,
}

impl RunCommand for Format {
    fn run(self, _config: Config) -> anyhow::Result<()> {
        let optimise = Optimise {
            paths: self.paths,
            output: self.output,
            config: None,
            recursive: self.recursive,
            hidden: self.hidden,
            threads: self.threads,
            extends: Some(Extends::None),
            pretty: self.pretty,
            space: self.space,
        };
        optimise.handle_paths(&Jobs::none())
    }
}
