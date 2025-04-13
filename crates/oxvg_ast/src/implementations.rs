//! XML document representations parsed by different implementations.
//!
//! You can create your own parser to build a tree for OXVG or use one of our
//! implementations for popular parsing libraries.
//! These parsers can be made available with the `"markup5ever"` or `"roxmltree"`
//! feature flags.
#[cfg(feature = "markup5ever")]
pub mod markup5ever;

#[cfg(feature = "roxmltree")]
pub mod roxmltree;

pub mod shared;
