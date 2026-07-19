//! Trait implementations of geometric calculations (i.e. [`geo::algorithm`]).
//!
//! Note, `Point`, `Line`, and `Rectangle` implement these via [`std::ops::Deref`].
mod area;
pub mod bool_ops;
pub mod coords_iter;
pub mod lines_iter;
