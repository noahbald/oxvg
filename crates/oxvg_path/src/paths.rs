//! Different path representations
#[cfg(feature = "geometry")]
pub mod events;
#[cfg(feature = "optimise")]
#[deprecated]
pub mod positioned;
#[cfg(feature = "geometry")]
pub mod segment;
pub mod svg;
