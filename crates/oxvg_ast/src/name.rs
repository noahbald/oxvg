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

    fn local_name(&self) -> Self::LocalName;

    fn prefix(&self) -> Option<Self::Prefix>;

    fn ns(&self) -> Self::Namespace;
}
