mod characters;
mod cursor;
mod diagnostics;
mod document;
mod file_reader;
mod markup;
mod references;
mod syntactic_constructs;
use crate::characters::char;
use crate::cursor::{Cursor, Span};
use crate::markup::{content, markup, ETag, Element, EmptyElemTag, NodeContent, STag, TagType};
use diagnostics::{SvgParseErrorMessage, SvgParseErrorProvider};
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
    match result {
        Ok(svg) => match &*svg.element.borrow() {
            Node::ContentNode(n) if n.2.tag_name.to_lowercase() == "svg" => Ok(()),
            Node::ContentNode((s_tag, ..)) => Err(SvgParseErrorProvider::new_error(
                s_tag.borrow().span.as_source_span(&file),
                NamedSource::new(config.path, file),
                SvgParseErrorMessage::NoRootElement,
            ))?,
            Node::EmptyNode(EmptyElemTag { span, .. }) => Err(SvgParseErrorProvider::new_error(
                span.as_source_span(&file),
                NamedSource::new(config.path, file),
                SvgParseErrorMessage::NoRootElement,
            ))?,
        },
        Err(error) => error.as_provider(config.path, &file),
    }
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
