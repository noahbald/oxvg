//! Traits and types for handling the command line arguments for OXVG.
use clap::{Parser, Subcommand};

use crate::{config::Config, optimise::Optimise};

/// A type for a runnable command.
pub trait RunCommand {
    /// Runs the command with the specified config.
    ///
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
/// Args that can be provided when running OXVG via the command line.
pub struct Args {
    #[clap(subcommand)]
    /// The subcommands for OXVG
    pub command: Command,
}

#[derive(Subcommand)]
/// The subcommands for OXVG
pub enum Command {
    /// Optimise SVG documents
    #[clap(alias = "optimize")]
    Optimise(Optimise),
}
