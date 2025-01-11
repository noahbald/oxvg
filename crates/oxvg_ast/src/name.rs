use std::{fmt::Display, hash::Hash};

use crate::atom::Atom;

pub trait Name:
    Eq
    + PartialEq
    + Clone
    + Default
    + std::fmt::Debug
    + for<'a> From<&'a str>
    + From<String>
    + 'static
    + Display
    + Hash
    + Ord
    + PartialOrd
{
    type LocalName: Atom;
    type Prefix: Atom;
    type Namespace: Atom;

    /// Returns the local part of the qualified name.
    fn local_name(&self) -> Self::LocalName;

    /// Returns the prefix of the qualified name.
    fn prefix(&self) -> Option<Self::Prefix>;

    /// Returns the namespace of the qualified name.
    fn ns(&self) -> Self::Namespace;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool;
}
