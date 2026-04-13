//! Different path representations
#[cfg(feature = "optimise")]
#[deprecated]
pub mod positioned;
#[cfg(feature = "geometry")]
pub mod segment;
pub mod svg;
