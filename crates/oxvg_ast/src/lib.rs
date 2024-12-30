#[macro_use]
extern crate bitflags;

#[cfg(feature = "style")]
#[macro_use]
extern crate smallvec;

pub mod atom;
pub mod attribute;
pub mod class_list;
pub mod document;
pub mod element;
pub mod implementations;
pub mod name;
pub mod node;

#[cfg(feature = "visitor")]
pub mod visitor;

#[cfg(feature = "parse")]
pub mod parse;

#[cfg(feature = "serialize")]
pub mod serialize;

#[cfg(feature = "selectors")]
pub mod selectors;

#[cfg(feature = "style")]
pub mod style;
