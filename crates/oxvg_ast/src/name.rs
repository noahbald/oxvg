//! XML qualified name types.
use std::{fmt::Display, hash::Hash};

use crate::atom::Atom;

/// A qualified name used for the names of tags and attributes.
pub trait Name:
    Eq + PartialEq + Clone + std::fmt::Debug + 'static + Hash + Ord + PartialOrd
{
    /// The local name (e.g. the `href` of `xlink:href`) of a qualified name.
    type LocalName: Atom;
    /// The prefix (e.g. `xlink` of `xlink:href`) of a qualified name.
    type Prefix: Atom;
    /// The resolved uri of the name
    type Namespace: Atom;

    /// Creates a qualified name from a prefix and local part
    fn new(prefix: Option<Self::Prefix>, local: Self::LocalName) -> Self;

    /// Returns the local part of the qualified name.
    fn local_name(&self) -> &Self::LocalName;

    /// Returns the prefix of the qualified name.
    fn prefix(&self) -> &Option<Self::Prefix>;

    /// Returns the namespace of the qualified name.
    fn ns(&self) -> &Self::Namespace;

    /// Returns the length of joining the prefix and local part of a name with a `:`
    fn len(&self) -> usize {
        match self.prefix() {
            Some(p) => p.len() + 1 + self.local_name().len(),
            None => self.local_name().len(),
        }
    }

    /// Returns whether the name is equivalent to an empty string
    fn is_empty(&self) -> bool {
        self.prefix().is_none() && self.local_name().is_empty()
    }

    /// Creates a qualified name from a string optionally seperating the
    /// prefix from a local-name with a `:`
    fn parse(value: &str) -> Self {
        let mut parts = value.split(':');
        let prefix_or_local = parts
            .next()
            .expect("Attempted to make qual-name from empty string");
        let maybe_local = parts.next().map(Into::into);
        assert_eq!(parts.next(), None);

        match maybe_local {
            Some(local) => Self::new(Some(prefix_or_local.into()), local),
            None => Self::new(None, prefix_or_local.into()),
        }
    }

    /// Calls `f` with a borrowed string, to prevent allocation in the case that
    /// the name doesn't have a prefix
    fn with_str<F, R>(&self, mut f: F) -> R
    where
        F: FnMut(&str) -> R,
    {
        match self.prefix() {
            Some(p) => {
                let string = format!("{p}:{}", self.local_name());
                f(string.as_str())
            }
            None => f(self.local_name().as_str()),
        }
    }

    /// returns a formatter to implement [Display] for a name
    fn formatter(&self) -> Formatter<'_, Self> {
        Formatter(self)
    }
}

/// Formats the contained qualified name
pub struct Formatter<'a, N: Name>(&'a N);

impl<'a, N: Name> Display for Formatter<'a, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.prefix() {
            Some(p) => f.write_fmt(format_args!("{p}:{}", self.0.local_name())),
            None => Display::fmt(&self.0.local_name(), f),
        }
    }
}
