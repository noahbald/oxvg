use std::{fmt::Display, hash::Hash};

use crate::atom::Atom;

pub trait Name:
    Eq + PartialEq + Clone + std::fmt::Debug + 'static + Hash + Ord + PartialOrd
{
    type LocalName: Atom;
    type Prefix: Atom;
    type Namespace: Atom;

    fn new(prefix: Option<Self::Prefix>, local: Self::LocalName) -> Self;

    /// Returns the local part of the qualified name.
    fn local_name(&self) -> &Self::LocalName;

    /// Returns the prefix of the qualified name.
    fn prefix(&self) -> &Option<Self::Prefix>;

    /// Returns the namespace of the qualified name.
    fn ns(&self) -> &Self::Namespace;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool;

    fn parse(value: &str) -> Self;

    fn formatter(&self) -> Formatter<'_, Self> {
        Formatter(self)
    }
}

pub struct Formatter<'a, N: Name>(&'a N);

impl<'a, N: Name> Display for Formatter<'a, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.prefix() {
            Some(p) => f.write_fmt(format_args!("{p}:{}", self.0.local_name())),
            None => Display::fmt(&self.0.local_name(), f),
        }
    }
}
