//! Oxvg is a toolchain for transforming, optimising, and linting SVG documents.

use clap::Parser;
use config::File;
use oxvg::{
    args::{Args, Command, RunCommand},
    config::Config,
};
use oxvg_ast::implementations::shared::Element;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config: Config<'static, Element> = config::Config::builder()
        .add_source(File::with_name("oxvgrc").required(false))
        .build()?
        .try_deserialize()?;

    match args.command {
        Command::Optimise(args) => args.run(config)?,
    }
    Ok(())
}
