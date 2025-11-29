//! Oxvg is a toolchain for transforming, optimising, and linting SVG documents.

use clap::Parser;
use oxvg::{
    args::{Args, Command, RunCommand},
    commands::Lint,
    config::Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    match args.command {
        Command::Optimise(args) => args.run(config).await,
        Command::Format(args) => args.run(config).await,
        Command::Lint(lint) => match lint {
            Lint::Check(args) => args.run(config).await,
            Lint::Serve(args) => args.run(config).await,
        },
    }
}
