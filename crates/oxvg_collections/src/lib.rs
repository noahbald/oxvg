//! Collections of data and types for SVG
//!
//! As opposed to `oxvg_ast`, which is for the structural types of an SVG document,
//! `oxvg_collections` is for the data types of an SVG document.
//!
//! # Types
//!
//! ## Atom
//!
//! The [`atom::Atom`] is used for storing string representations that may be provided by
//! the parser.
//! It may be used in the values, names, and text-content of a document.
//!
//! ## Names
//!
//! A [`name::QualName`] consists of a [`name::Prefix`] with an associated [`name::NS`] and a local-name
//! represented by an [`atom::Atom`].
//!
//! The names can be used for the name of an [`attribute::AttrId`] or [`element::ElementId`].
//!
//! ## Attributes
//!
//! An [`attribute::Attr`] is a data point that can be assigned to an [`element::ElementId`]. Attributes
//! described in the SVG have an associated [`content_type::ContentType`].
//!
//! ## Elements
//!
//! An [`element::ElementId`] can be thought of as a *class* of an SVG document with associated
//! attributes and child elements.
//! SVG elements are specified to only allow specific attributes and elements for each element type.

#[macro_use]
extern crate bitflags;

pub mod atom;
pub mod attribute;
pub mod content_type;
pub mod element;
pub mod name;
