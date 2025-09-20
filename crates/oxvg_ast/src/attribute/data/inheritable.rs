use crate::{
    error::{ParseError, PrinterError},
    parse::{Parse, Parser},
    serialize::{Printer, ToAtom},
};
use std::fmt::{Debug, Write};

#[derive(Clone, Debug, PartialEq)]
pub enum Inheritable<T: Clone + Debug + PartialEq> {
    Defined(T),
    Inherited,
}
impl<T: Clone + Debug + PartialEq> Inheritable<T> {
    pub fn map<'a, U: Clone + Debug + PartialEq, F>(&'a self, f: F) -> Inheritable<U>
    where
        F: FnOnce(&'a T) -> U,
    {
        match self {
            Self::Defined(x) => Inheritable::Defined(f(x)),
            Self::Inherited => Inheritable::Inherited,
        }
    }
}
impl<'input, T: Parse<'input> + Clone + Debug + PartialEq> Parse<'input> for Inheritable<T> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("inherit")
                    .map(|_| Self::Inherited)
            })
            .or_else(|_| T::parse(input).map(Self::Defined))
    }
}
impl<T: ToAtom + Clone + Debug + PartialEq> Inheritable<Box<T>> {
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
impl<T: ToAtom + Clone + Debug + PartialEq> ToAtom for Inheritable<T> {
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
