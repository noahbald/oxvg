mod diagnostics;
mod document;
mod file_reader;
mod state;
mod syntactic_constructs;
use diagnostics::SVGErrors;
use document::Document;
use miette::{NamedSource, Result};
use std::env;
use std::fs;
use std::process;

fn main() -> Result<()> {
    let config = Config::make(env::args()).unwrap_or_else(|error| {
        eprintln!("Invalid arguments: {error}");
        process::exit(1);
    });
    let file = fs::read_to_string(&config.path).expect("Unable to read file");
    let result = Document::parse(&file);
    SVGErrors::from_errors(NamedSource::new(config.path, file), result.errors).emit()
}

struct Config {
    path: String,
}

impl Config {
    pub fn make(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        if let Some(path) = args.next() {
            Ok(Config { path })
        } else {
            Err("No path given")
        }
    }
}
