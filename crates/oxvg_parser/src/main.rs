use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use config::{Config, File};
use rcdom::SerializableHandle;
use serde::Deserialize;
use xml5ever::{
    driver::{parse_document, XmlParseOpts},
    serialize::{serialize, SerializeOpts},
    tendril::TendrilSink,
};

trait CommandArgs {
    fn run(&self, config: OxvgConfig);
}

#[derive(Deserialize, Default)]
struct OxvgConfig {
    optimisation: Option<oxvg_optimiser::Jobs>,
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
struct OxvgArgs {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Optimise SVG documents
    #[clap(alias = "optimize")]
    Optimise(OptimiseArgs),
}

#[derive(Args)]
pub struct OptimiseArgs {
    /// The target paths to optimise
    #[clap(value_parser, default_value = ".")]
    pub paths: Vec<PathBuf>,
    /// The file or directory to output results to.
    /// Defaults to stdout
    #[clap(long = "output", short = 'o')]
    pub output: Option<PathBuf>,
}

impl CommandArgs for OptimiseArgs {
    fn run(&self, config: OxvgConfig) {
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

fn load_files(paths: &[PathBuf]) -> Vec<(PathBuf, Vec<u8>)> {
    paths.iter().flat_map(load_file).collect()
}

fn load_file(path: &PathBuf) -> Box<dyn Iterator<Item = (PathBuf, Vec<u8>)>> {
    let metadata = std::fs::metadata(path).unwrap();
    if metadata.is_symlink() {
        return load_file(&std::fs::read_link(path).unwrap());
    };
    if metadata.is_file() {
        return Box::new(vec![(path.clone(), std::fs::read(path).unwrap())].into_iter());
    }
    Box::new(
        std::fs::read_dir(path)
            .unwrap()
            .map(|dir| dir.unwrap().path())
            .filter(|path| path.ends_with(".svg"))
            .map(|path| (path.clone(), std::fs::read(path.clone()).unwrap())),
    )
}

fn write_file(path: &Option<PathBuf>, source: &PathBuf, dom: &rcdom::RcDom) {
    let document: SerializableHandle = dom.document.clone().into();
    let Some(path) = path else {
        serialize(&mut std::io::stdout(), &document, SerializeOpts::default()).unwrap();
        return;
    };

    let metadata = std::fs::metadata(path).ok();
    if metadata.clone().is_some_and(|data| data.is_symlink()) {
        return write_file(&Some(path.clone()), source, dom);
    };

    let mut sink = if metadata.is_some_and(|data| data.is_dir()) {
        let path = path.join(source.file_name().unwrap());
        std::fs::File::create(path).unwrap()
    } else {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::File::create(path).unwrap()
    };
    serialize(&mut sink, &document, SerializeOpts::default()).unwrap();
}

fn main() -> anyhow::Result<()> {
    let args = OxvgArgs::parse();
    let config: OxvgConfig = Config::builder()
        .add_source(File::with_name("oxvgrc").required(false))
        .build()?
        .try_deserialize()?;

    match args.command {
        Command::Optimise(args) => args.run(config),
    }
    Ok(())
}
