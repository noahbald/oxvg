mod characters;
mod cursor;
mod diagnostics;
mod document;
mod file_reader;
mod markup;
mod references;
mod state;
mod syntactic_constructs;
use crate::characters::char;
use crate::cursor::{Cursor, Span};
use crate::markup::{content, ETag, Element, EmptyElemTag, NodeContent, STag};
use diagnostics::{SVGErrorLabel, SVGErrors};
use document::{Document, Node};
use markup::Markup;
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
    dbg!(&result.errors);
    SVGErrors::from_errors(NamedSource::new(config.path, file), result.errors).emit()
}

struct Config {
    path: String,
}

impl Config {
    pub fn make(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        let path = match args.next() {
            Some(arg) => arg,
            None => return Err("No path given"),
        };
        Ok(Config { path })
    }
}
