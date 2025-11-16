//! OXVG uses an ast representation of an XML document for processing SVGs.
//!
//! As opposed to `oxvg_collections`, which is for the data types of an SVG document,
//! `oxvg_ast` is for the structural types of an SVG document.
//!
//! # Types
//!
//! There are three major types for AST representation
//!
//! - [`node::Node`] for data nodes of an SVG document.
//! - [`element::Element`] for a node that contains SVG element data.
//! - [`attribute::Attribute`] for attributes of an SVG element.
//!
//! # Features
//!
//! ## Parsing
//!
//! When a parser such as [`parse::markup5ever`] or [`parse::roxmltree`] feature flag is enabled,
//! you can bring the respective parsing library and use that to build an OXVG AST representation.
//!
//! ## Visitors
//!
//! When you enable the [`visitor`] feature flag, you can use the [`visitor::Visitor`] trait to
//! implement a type that can process each node of the document.
//!
//! ## Serialize
//!
//! When you enable the [`serialize`] feature flag, you can use the [`serialize::Node`] method on
//! a [`node::Node`] or [`element::Element`] in conjunction with [`xmlwriter::XmlWriter`] to
//! write an SVG document to a buffer or string.
#[cfg(feature = "visitor")]
#[macro_use]
extern crate bitflags;

pub mod class_list;
pub mod document;

#[cfg(feature = "parse")]
pub mod parse;

pub mod arena;
pub mod attribute;
pub mod element;
pub mod error;
pub mod node;

#[cfg(feature = "visitor")]
pub mod visitor;

#[cfg(feature = "serialize")]
pub mod serialize;
#[cfg(feature = "serialize")]
pub mod xmlwriter;

#[cfg(feature = "selectors")]
pub mod selectors;

pub mod style;
