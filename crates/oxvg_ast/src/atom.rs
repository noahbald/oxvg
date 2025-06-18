//! XML atom traits.
use std::{fmt::Display, hash::Hash, ops::Deref};

/// A string representation
/// It may be an interned string, enum representation, or a mix of both
pub trait Atom:
    Eq
    + Display
    + PartialEq
    + Ord
    + std::fmt::Debug
    + Clone
    + Default
    + for<'a> From<&'a str>
    + From<String>
    + AsRef<str>
    + Deref<Target = str>
    + Hash
    + 'static
{
    /// Extracts the string slice of the atom
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl<T> Atom for T where
    T: Eq
        + Display
        + PartialEq
        + Ord
        + std::fmt::Debug
        + Clone
        + Default
        + for<'a> From<&'a str>
        + From<String>
        + AsRef<str>
        + Deref<Target = str>
        + Hash
        + 'static
{
}
