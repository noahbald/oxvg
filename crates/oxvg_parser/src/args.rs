use std::path::PathBuf;

use clap::{Parser, Subcommand};
use xml5ever::{
    driver::{parse_document, XmlParseOpts},
    tendril::TendrilSink,
};

use crate::{
    config::Config,
    fs::{load_files, write_file},
};

pub trait RunCommand {
    fn run(&self, config: Config);
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
    fn run(&self, config: Config) {
        let files = load_files(&self.paths);
        for (path, file) in &files {
            let dom: rcdom::RcDom =
                parse_document(rcdom::RcDom::default(), XmlParseOpts::default())
                    .one(String::from_utf8_lossy(file).to_string());
            config.optimisation.clone().unwrap_or_default().run(&dom);
            write_file(&self.output, path, &dom);
        }
    }
}
