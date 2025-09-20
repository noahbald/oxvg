//! Collection for attributes when speficied as a list
use std::ops::Deref;

use crate::{
    error::ParseError,
    parse::{Parse, Parser},
    serialize::ToAtom,
};

#[derive(Clone, Debug, PartialEq, Eq)]
/// A `' '` delimiter
pub struct Space;
#[derive(Clone, Debug, PartialEq, Eq)]
/// A `','` delimiter
pub struct Comma;
#[derive(Clone, Debug, PartialEq, Eq)]
/// A `' '` or `','` delimiter
pub struct SpaceOrComma;
#[derive(Clone, Debug, PartialEq, Eq)]
/// A `';'` delimiter
pub struct Semicolon;

#[derive(Clone, Debug, PartialEq, Eq)]
/// A type of well-known delimiter
pub enum Seperators {
    /// A `' '` delimiter
    Space,
    /// A `','` delimiter
    Comma,
    /// A `' '` or `','` delimiter
    SpaceOrComma,
    /// A `';'` delimiter
    Semicolon,
}
/// A trait for seperators of [`ListOf`]
pub trait Seperator: Clone + ToAtom {
    /// Returns whether whitespace is intrinsic to this seperator
    fn maybe_skip_whitespace(_input: &mut Parser<'_, '_>) {}
    /// Constructs this seperator
    fn new() -> Self;
    /// Returns an enumerable instance of seperators
    fn id(&self) -> Seperators;
    /// Parses the seperator
    ///
    /// # Errors
    /// If the parser fails
    fn parse<'input>(input: &mut Parser<'input, '_>) -> Result<(), ParseError<'input>>;
}
impl Seperator for Space {
    fn id(&self) -> Seperators {
        Seperators::Space
    }
    fn new() -> Self {
        Self
    }
    fn parse<'input>(input: &mut Parser<'input, '_>) -> Result<(), ParseError<'input>> {
        input.expect_whitespace()?;
        input.skip_whitespace();
        Ok(())
    }
}
impl ToAtom for Space {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(' ')
    }
}
impl Seperator for Comma {
    fn maybe_skip_whitespace(input: &mut Parser<'_, '_>) {
        input.skip_whitespace();
    }
    fn new() -> Self {
        Self
    }
    fn id(&self) -> Seperators {
        Seperators::Comma
    }
    fn parse<'input>(input: &mut Parser<'input, '_>) -> Result<(), ParseError<'input>> {
        input.expect_comma()?;
        Ok(())
    }
}
impl ToAtom for Comma {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(',')
    }
}
impl Seperator for SpaceOrComma {
    fn id(&self) -> Seperators {
        Seperators::SpaceOrComma
    }
    fn new() -> Self {
        Self
    }
    fn parse<'input>(input: &mut Parser<'input, '_>) -> Result<(), ParseError<'input>> {
        if input.try_parse(Parser::expect_whitespace).is_ok() {
            input.skip_whitespace();
            input.try_parse(Parser::expect_comma).ok();
            input.skip_whitespace();
            return Ok(());
        }
        Comma::parse(input)?;
        input.skip_whitespace();
        Ok(())
    }
}
impl ToAtom for SpaceOrComma {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(' ')
    }
}
impl Seperator for Semicolon {
    fn maybe_skip_whitespace(input: &mut Parser<'_, '_>) {
        input.skip_whitespace();
    }
    fn new() -> Self {
        Self
    }
    fn id(&self) -> Seperators {
        Seperators::Semicolon
    }
    fn parse<'input>(input: &mut Parser<'input, '_>) -> Result<(), ParseError<'input>> {
        input.expect_semicolon()?;
        Ok(())
    }
}
impl ToAtom for Semicolon {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(';')
    }
}
impl Seperator for Seperators {
    fn maybe_skip_whitespace(_input: &mut Parser<'_, '_>) {
        unreachable!()
    }
    fn new() -> Self {
        unreachable!()
    }
    fn id(&self) -> Seperators {
        self.clone()
    }
    fn parse<'input>(_input: &mut Parser<'input, '_>) -> Result<(), ParseError<'input>> {
        unreachable!()
    }
}
impl ToAtom for Seperators {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Space => Space.write_atom(dest),
            Self::Comma => Comma.write_atom(dest),
            Self::SpaceOrComma => SpaceOrComma.write_atom(dest),
            Self::Semicolon => Semicolon.write_atom(dest),
        }
    }
}

/// A list of values which are read or written with a specific delimiter
#[derive(Debug, PartialEq)]
pub struct ListOf<T: std::fmt::Debug + PartialEq, S: Seperator> {
    /// A list of values seperated by a seperator
    pub list: Vec<T>,
    /// A delimiter that can be used between each item of the list
    pub seperator: S,
}

impl<T: Clone + std::fmt::Debug + PartialEq, S: Seperator> Clone for ListOf<T, S> {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            seperator: self.seperator.clone(),
        }
    }
}

impl<T: std::fmt::Debug + PartialEq, S: Seperator> Deref for ListOf<T, S> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl<'input, T: Parse<'input> + std::fmt::Debug + PartialEq, S: Seperator> Parse<'input>
    for ListOf<T, S>
{
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let mut list = Vec::with_capacity(1);
        loop {
            S::maybe_skip_whitespace(input);
            let Ok(item) = input.try_parse(T::parse) else {
                break;
            };
            list.push(item);
            if S::parse(input).is_err() {
                break;
            }
        }
        Ok(Self {
            list,
            seperator: S::new(),
        })
    }
}
impl<T: ToAtom + std::fmt::Debug + PartialEq, S: Seperator> ListOf<Box<T>, S> {
    /// Serialize self into CSS or an attribute value
    ///
    /// # Errors
    /// If the printer fails.
    pub fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        let mut iter = self.list.iter();
        if let Some(t) = iter.next() {
            t.write_atom(dest)?;
        }
        for t in iter {
            self.seperator.write_atom(dest)?;
            t.write_atom(dest)?;
        }
        Ok(())
    }
}
impl<T: ToAtom + std::fmt::Debug + PartialEq, S: Seperator> ToAtom for ListOf<T, S> {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        let mut iter = self.list.iter();
        if let Some(t) = iter.next() {
            t.write_atom(dest)?;
        }
        for t in iter {
            self.seperator.write_atom(dest)?;
            t.write_atom(dest)?;
        }
        Ok(())
    }
}

impl<T: std::fmt::Debug + PartialEq, S: Seperator> ListOf<T, S> {
    /// Returns self with a new list of items, as applied by the given function over each item
    pub fn map<'a, U: std::fmt::Debug + PartialEq, F>(&'a self, f: F) -> ListOf<U, S>
    where
        F: FnMut(&'a T) -> U,
    {
        ListOf {
            list: self.list.iter().map(f).collect(),
            seperator: self.seperator.clone(),
        }
    }

    /// Returns self with a new list of mutable items, as applied by the given function over each item
    pub fn map_mut<'a, U: std::fmt::Debug + PartialEq, F>(&'a mut self, f: F) -> ListOf<U, S>
    where
        F: FnMut(&'a mut T) -> U,
    {
        ListOf {
            list: self.list.iter_mut().map(f).collect(),
            seperator: self.seperator.clone(),
        }
    }

    /// Returns self with a new seperator, as applied by the given function
    pub fn map_sep<U: Seperator, F>(self, f: F) -> ListOf<T, U>
    where
        F: FnOnce(S) -> U,
    {
        ListOf {
            list: self.list,
            seperator: f(self.seperator),
        }
    }
}
