//! Analyses SVG documents to find common errors
pub mod error;
#[cfg(feature = "parse")]
mod parse;
mod rules;
mod utils;

pub use rules::{Rules, Severity};
