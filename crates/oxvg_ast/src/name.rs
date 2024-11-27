use std::fmt::Display;

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
}
