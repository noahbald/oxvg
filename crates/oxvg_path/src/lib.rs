//! OXVG Path is a library used for parsing and minifying SVG paths.
//! It supports parsing of any valid SVG path and provides optimisations close to exactly as SVGO.
//!
//! Use the [`Path`] struct for simple parsing and serializing. By solely parsing and serializing,
//! it will produce a path with prettier formatting.
//! A path is made up individual command [`Data`](command::Data).
//!
//! For more rigorous minification, try using the [`Path::optimize`] method. This will use
//! non-destructive conversions to shorten the path.
//!
//! For a simpler path representation, best for building and manipulations, use [`paths::segment::Path`].
//!
//! # Licensing
//!
//! This library is based off the [`convertPathData`](https://svgo.dev/docs/plugins/convertPathData/) plugin from SVGO and is similarly released under MIT.
#[cfg(feature = "optimise")]
#[cfg(feature = "parse")]
#[macro_use]
extern crate bitflags;

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

#[cfg(feature = "algorithm")]
pub mod algorithm;
#[cfg(feature = "format")]
mod format;
#[cfg(feature = "geometry")]
pub mod geometry;
#[cfg(feature = "geometry")]
mod gjk_intersection;
#[cfg(feature = "geometry")]
pub(crate) mod math;
#[cfg(feature = "optimise")]
pub mod optimize;
#[cfg(feature = "parse")]
pub mod parser;
pub mod paths;

pub use paths::svg::{command, Path};
