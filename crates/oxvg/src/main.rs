//! Oxvg is a toolchain for transforming, optimising, and linting SVG documents.

use clap::Parser;
use oxvg::{
    args::{Args, Command, RunCommand},
    config::Config,
};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    match args.command {
        Command::Optimise(args) => args.run(config)?,
        Command::Format(args) => args.run(config)?,
    }
    Ok(())
}
