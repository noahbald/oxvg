use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::Config,
    fs::{load_files, write_file},
};

pub trait RunCommand {
    /// # Errors
    ///
    /// If any part of the lifecycle fails
    /// * Fails to read or parse any files
    /// * Fails to write or serialize to any files
    fn run(&self, config: Config) -> anyhow::Result<()>;
}

#[derive(Parser)]
#[clap(
    bin_name = "oxvg",
    name="oxvg",
    author,
    version,
    about = "Your versatile vector-graphics toolchain",
    long_about = None
)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Optimise SVG documents
    #[clap(alias = "optimize")]
    Optimise(Optimise),
}

#[derive(clap::Args)]
pub struct Optimise {
    /// The target paths to optimise
    #[clap(value_parser, default_value = ".")]
    pub paths: Vec<PathBuf>,
    /// The file or directory to output results to.
    /// Defaults to stdout
    #[clap(long = "output", short = 'o')]
    pub output: Option<PathBuf>,
}

impl RunCommand for Optimise {
    fn run(&self, config: Config) -> anyhow::Result<()> {
        use oxvg_ast::{implementations::markup5ever::Node5Ever, parse::Node};

        let files = load_files(&self.paths);
        let config = config.optimisation.unwrap_or_default();
        for (path, file) in &files {
            let dom: Node5Ever = Node::parse(&String::from_utf8_lossy(file))?;
            config.clone().run(&dom)?;
            write_file(&self.output, path, &dom);
        }
        Ok(())
    }
}
