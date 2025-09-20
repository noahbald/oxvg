//! OXVG uses an ast representation of an XML document for processing SVGs.
#[cfg(feature = "visitor")]
#[macro_use]
extern crate bitflags;

pub mod class_list;
pub mod document;

pub mod parse;

pub mod arena;
pub mod atom;
pub mod attribute;
pub mod element;
pub mod error;
pub mod name;
pub mod node;

#[cfg(feature = "visitor")]
pub mod visitor;

pub mod serialize;
#[cfg(feature = "serialize")]
pub mod xmlwriter;

#[cfg(feature = "selectors")]
pub mod selectors;

pub mod style;
