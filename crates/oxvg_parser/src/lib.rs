mod diagnostics;
mod document;
mod file_reader;
mod state;
mod syntactic_constructs;

pub use crate::{
    diagnostics::{SVGError, SVGErrors},
    document::Document,
    file_reader::{Child, Element, Parent},
};
