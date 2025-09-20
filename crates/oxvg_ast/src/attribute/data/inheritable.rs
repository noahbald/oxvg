//! A container for attributes which accept and `"inherited"` value
use crate::{
    error::{ParseError, PrinterError},
    parse::{Parse, Parser},
    serialize::{Printer, ToAtom},
};
use std::fmt::{Debug, Write};

#[derive(Debug, PartialEq)]
/// Wraps a type that can be provided with `"inherited"` as a value
pub enum Inheritable<T: Debug + PartialEq> {
    /// The value of the attribute is of type `T`
    Defined(T),
    /// The value of the attribute is `"inherited"`
    Inherited,
}
impl<T: Clone + Debug + PartialEq> Clone for Inheritable<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Defined(t) => Self::Defined(t.clone()),
            Self::Inherited => Self::Inherited,
        }
    }
}
impl<T: Debug + PartialEq> Inheritable<T> {
    /// Returns self with a new value, as applied by the given function over the item
    pub fn map<'a, U: Debug + PartialEq, F>(&'a self, f: F) -> Inheritable<U>
    where
        F: FnOnce(&'a T) -> U,
    {
        match self {
            Self::Defined(x) => Inheritable::Defined(f(x)),
            Self::Inherited => Inheritable::Inherited,
        }
    }

    /// Returns self with a new value, as applied by the given function over the item
    pub fn map_mut<'a, U: Debug + PartialEq, F>(&'a mut self, f: F) -> Inheritable<U>
    where
        F: FnOnce(&'a mut T) -> U,
    {
        match self {
            Self::Defined(x) => Inheritable::Defined(f(x)),
            Self::Inherited => Inheritable::Inherited,
        }
    }

    /// Returns the inner value as an option, where an inherited value is [`None`]
    pub fn option(self) -> Option<T> {
        match self {
            Self::Defined(x) => Some(x),
            Self::Inherited => None,
        }
    }

    /// Returns a reference to the inner value as an option, where an inherited value is [`None`]
    pub fn option_ref(&self) -> Option<&T> {
        match self {
            Self::Defined(x) => Some(x),
            Self::Inherited => None,
        }
    }

    /// Mutably returns the inner value as an option, where an inherited value is [`None`]
    pub fn option_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Defined(x) => Some(x),
            Self::Inherited => None,
        }
    }
}
impl<'input, T: Parse<'input> + Debug + PartialEq> Parse<'input> for Inheritable<T> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("inherit")
                    .map(|()| Self::Inherited)
            })
            .or_else(|_| T::parse(input).map(Self::Defined))
    }
}
impl<T: ToAtom + Debug + PartialEq> Inheritable<Box<T>> {
    /// See [`crate::serialize::ToAtom::write_atom`]
    ///
    /// # Errors
    /// If the printer fails
    pub fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: Write,
    {
        match self {
            Self::Inherited => dest.write_str("inherit"),
            Self::Defined(inner) => inner.write_atom(dest),
        }
    }
}
impl<T: ToAtom + Debug + PartialEq> ToAtom for Inheritable<T> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: Write,
    {
        match self {
            Self::Inherited => dest.write_str("inherit"),
            Self::Defined(inner) => inner.write_atom(dest),
        }
    }
}

/// Like `Inheritable::option_mut` when contained within `RefMut`.
pub fn map_ref_mut<T: Debug + PartialEq>(
    inheritable: std::cell::RefMut<Inheritable<T>>,
) -> Option<std::cell::RefMut<T>> {
    std::cell::RefMut::filter_map(inheritable, Inheritable::option_mut).ok()
}
