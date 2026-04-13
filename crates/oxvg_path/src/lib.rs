//! OXVG Path is a library used for parsing and minifying SVG paths.
//! It supports parsing of any valid SVG path and provides optimisations close to exactly as SVGO.
//!
//! Use the [Path] struct for simple parsing and serializing. By only parsing and serializing,
//! it will produce optimised formatting out of the box.
//! It is made up individual command [Data](command::Data).
//!
//! For more rigorous minification, try using the [run](convert::run) function. This will use
//! non-destructive conversions to shorten the path.
//!
//! # Differences to SVGO
//!
//! - Unlike SVGO, all close paths are serialized as `Z` instead of either `z` or `Z`. This is fine because the two commands function exactly the same.
//! - An equivalent of the `applyTransforms` option isn't available, but may be in the future.
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

#[cfg(feature = "optimise")]
#[deprecated]
pub mod convert;
#[cfg(feature = "format")]
mod format;
#[cfg(feature = "geometry")]
pub mod geometry;
#[cfg(feature = "geometry")]
#[deprecated]
mod gjk_intersection;
#[cfg(feature = "geometry")]
pub(crate) mod math;
#[cfg(feature = "parse")]
pub mod parser;
pub mod paths;
#[cfg(feature = "optimise")]
#[deprecated = "Use [`crate::geometry::Point`] instead"]
pub mod points;
#[cfg(feature = "optimise")]
#[deprecated]
mod position;

pub use paths::svg::{command, Path};
