use clap::{Parser, Subcommand};

use crate::{config::Config, optimise::Optimise};

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
    name = "oxvg",
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
