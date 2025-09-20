use std::ops::Deref;

use crate::{error::ParseErrorKind, parse::Parse, serialize::ToAtom};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Space;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comma;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceOrComma;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Semicolon;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seperators {
    Space,
    Comma,
    SpaceOrComma,
    Semicolon,
}
pub trait Seperator: Clone + ToAtom {
    fn maybe_skip_whitespace(_input: &mut cssparser_lightningcss::Parser<'_, '_>) {}
    fn new() -> Self;
    fn id(&self) -> Seperators;
    fn parse<'input, 't>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<(), cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>>;
}
impl Seperator for Space {
    fn id(&self) -> Seperators {
        Seperators::Space
    }
    fn new() -> Self {
        Self
    }
    fn parse<'input, 't>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<(), cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
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
    fn maybe_skip_whitespace(input: &mut cssparser_lightningcss::Parser<'_, '_>) {
        input.skip_whitespace();
    }
    fn new() -> Self {
        Self
    }
    fn id(&self) -> Seperators {
        Seperators::Comma
    }
    fn parse<'input, 't>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<(), cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
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
    fn parse<'input, 't>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<(), cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        if input.try_parse(|input| input.expect_whitespace()).is_ok() {
            input.skip_whitespace();
            return Ok(());
        }
        Comma::parse(input)
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
    fn maybe_skip_whitespace(input: &mut cssparser_lightningcss::Parser<'_, '_>) {
        input.skip_whitespace();
    }
    fn new() -> Self {
        Self
    }
    fn id(&self) -> Seperators {
        Seperators::Semicolon
    }
    fn parse<'input, 't>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<(), cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
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
    fn maybe_skip_whitespace(_input: &mut cssparser_lightningcss::Parser<'_, '_>) {
        unreachable!()
    }
    fn new() -> Self {
        unreachable!()
    }
    fn id(&self) -> Seperators {
        self.clone()
    }
    fn parse<'input, 't>(
        _input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<(), cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
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

#[derive(Clone, Debug, PartialEq)]
pub struct ListOf<T: Clone + std::fmt::Debug + PartialEq, S: Seperator> {
    pub list: Vec<T>,
    pub seperator: S,
}

impl<'input, T: Clone + std::fmt::Debug + PartialEq, S: Seperator> Deref for ListOf<T, S> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl<'input, T: Parse<'input> + Clone + std::fmt::Debug + PartialEq, S: Seperator> Parse<'input>
    for ListOf<T, S>
{
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
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
impl<'input, T: ToAtom + Clone + std::fmt::Debug + PartialEq, S: Seperator> ListOf<Box<T>, S> {
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
impl<'input, T: ToAtom + Clone + std::fmt::Debug + PartialEq, S: Seperator> ToAtom
    for ListOf<T, S>
{
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

impl<T: Clone + std::fmt::Debug + PartialEq, S: Seperator> ListOf<T, S> {
    pub fn map<'a, U: Clone + std::fmt::Debug + PartialEq, F>(&'a self, f: F) -> ListOf<U, S>
    where
        F: FnMut(&'a T) -> U,
    {
        ListOf {
            list: self.list.iter().map(f).collect(),
            seperator: self.seperator.clone(),
        }
    }

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
