//! Collection for attributes when specified as a list
use std::ops::Deref;

#[cfg(feature = "parse")]
use oxvg_parse::{error::Error, Parse, Parser};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};

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
pub enum Separators {
    /// A `' '` delimiter
    Space,
    /// A `','` delimiter
    Comma,
    /// A `' '` or `','` delimiter
    SpaceOrComma,
    /// A `';'` delimiter
    Semicolon,
}
#[cfg(not(feature = "serialize"))]
trait SeparatorBound {}
#[cfg(feature = "serialize")]
trait SeparatorBound: ToValue {}
#[cfg(not(feature = "serialize"))]
impl<T> SeparatorBound for T {}
#[cfg(feature = "serialize")]
impl<T: ToValue> SeparatorBound for T {}
/// A trait for separators of [`ListOf`]
#[allow(private_bounds)]
pub trait Separator: Clone + SeparatorBound {
    #[cfg(feature = "parse")]
    /// Returns whether whitespace is intrinsic to this separator
    fn maybe_skip_whitespace(_input: &mut Parser<'_>) {}
    /// Constructs this separator
    fn new() -> Self;
    /// Returns an enumerable instance of separators
    fn id(&self) -> Separators;
    #[cfg(feature = "parse")]
    /// Parses the separator
    ///
    /// # Errors
    /// If the parser fails
    fn parse<'input>(input: &mut Parser<'input>) -> Result<(), Error<'input>> {
        input
            .expect_matches("delim", |char| Ok(Self::matches(char)))
            .map(|_| ())
    }
    /// Returns whether the character matches the separator
    fn matches(char: char) -> bool;
}
impl Separator for Space {
    fn id(&self) -> Separators {
        Separators::Space
    }
    fn new() -> Self {
        Self
    }
    fn matches(char: char) -> bool {
        char.is_whitespace()
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Space {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(' ')
    }
}
impl Separator for Comma {
    #[cfg(feature = "parse")]
    fn maybe_skip_whitespace(input: &mut Parser<'_>) {
        input.skip_whitespace();
    }
    fn new() -> Self {
        Self
    }
    fn id(&self) -> Separators {
        Separators::Comma
    }
    fn matches<'input>(char: char) -> bool {
        char == ','
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Comma {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(',')
    }
}
impl Separator for SpaceOrComma {
    fn id(&self) -> Separators {
        Separators::SpaceOrComma
    }
    fn new() -> Self {
        Self
    }
    #[cfg(feature = "parse")]
    fn parse<'input>(input: &mut Parser<'input>) -> Result<(), Error<'input>> {
        if input.try_parse(Parser::expect_whitespace).is_ok() {
            input.skip_whitespace();
            input.skip_char(',');
            input.skip_whitespace();
            return Ok(());
        }
        Comma::parse(input)?;
        input.skip_whitespace();
        Ok(())
    }
    fn matches<'input>(char: char) -> bool {
        char.is_whitespace() || char == ','
    }
}
#[cfg(feature = "serialize")]
impl ToValue for SpaceOrComma {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(' ')
    }
}
impl Separator for Semicolon {
    #[cfg(feature = "parse")]
    fn maybe_skip_whitespace(input: &mut Parser<'_>) {
        input.skip_whitespace();
    }
    fn new() -> Self {
        Self
    }
    fn id(&self) -> Separators {
        Separators::Semicolon
    }
    fn matches<'input>(char: char) -> bool {
        char == ';'
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Semicolon {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_char(';')
    }
}
impl Separator for Separators {
    #[cfg(feature = "parse")]
    fn maybe_skip_whitespace(_input: &mut Parser<'_>) {
        unreachable!()
    }
    fn new() -> Self {
        unreachable!()
    }
    fn id(&self) -> Separators {
        self.clone()
    }
    #[cfg(feature = "parse")]
    fn parse<'input>(_input: &mut Parser<'input>) -> Result<(), Error<'input>> {
        unreachable!()
    }
    fn matches<'input>(_char: char) -> bool {
        unreachable!()
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Separators {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Space => Space.write_value(dest),
            Self::Comma => Comma.write_value(dest),
            Self::SpaceOrComma => SpaceOrComma.write_value(dest),
            Self::Semicolon => Semicolon.write_value(dest),
        }
    }
}

/// A list of values which are read or written with a specific delimiter
#[derive(Debug, PartialEq)]
pub struct ListOf<T: std::fmt::Debug + PartialEq, S: Separator> {
    /// A list of values separated by a separator
    pub list: Vec<T>,
    /// A delimiter that can be used between each item of the list
    pub separator: S,
}

impl<T: Clone + std::fmt::Debug + PartialEq, S: Separator> Clone for ListOf<T, S> {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            separator: self.separator.clone(),
        }
    }
}

impl<T: std::fmt::Debug + PartialEq, S: Separator> Deref for ListOf<T, S> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

#[cfg(feature = "parse")]
impl<'input, T: Parse<'input> + std::fmt::Debug + PartialEq, S: Separator> Parse<'input>
    for ListOf<T, S>
{
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let mut start = Parser::new(input.take_matches(|char| !S::matches(char)));
        let mut list = match T::parse(&mut start) {
            Ok(first) if start.is_empty() => vec![first],
            Ok(_) => return Err(Error::ExpectedDone),
            Err(_) if start.is_empty() => {
                return Ok(Self {
                    list: vec![],
                    separator: S::new(),
                })
            }
            Err(e) => return Err(e),
        };
        loop {
            if S::parse(input).is_err() {
                break;
            }
            S::maybe_skip_whitespace(input);
            list.push(T::parse_string(
                input.take_matches(|char| !S::matches(char)),
            )?);
        }
        Ok(Self {
            list,
            separator: S::new(),
        })
    }
}
#[cfg(feature = "serialize")]
impl<T: ToValue + std::fmt::Debug + PartialEq, S: Separator> ListOf<Box<T>, S> {
    /// Serialize self into CSS or an attribute value
    ///
    /// # Errors
    /// If the printer fails.
    pub fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let mut iter = self.list.iter();
        if let Some(t) = iter.next() {
            t.write_value(dest)?;
        }
        for t in iter {
            self.separator.write_value(dest)?;
            t.write_value(dest)?;
        }
        Ok(())
    }
}
#[cfg(feature = "serialize")]
impl<T: ToValue + std::fmt::Debug + PartialEq, S: Separator> ToValue for ListOf<T, S> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let mut iter = self.list.iter();
        if let Some(t) = iter.next() {
            t.write_value(dest)?;
        }
        for t in iter {
            self.separator.write_value(dest)?;
            t.write_value(dest)?;
        }
        Ok(())
    }
}

impl<T: std::fmt::Debug + PartialEq, S: Separator> ListOf<T, S> {
    /// Returns self with a new list of items, as applied by the given function over each item
    pub fn map<'a, U: std::fmt::Debug + PartialEq, F>(&'a self, f: F) -> ListOf<U, S>
    where
        F: FnMut(&'a T) -> U,
    {
        ListOf {
            list: self.list.iter().map(f).collect(),
            separator: self.separator.clone(),
        }
    }

    /// Returns self with a new list of mutable items, as applied by the given function over each item
    pub fn map_mut<'a, U: std::fmt::Debug + PartialEq, F>(&'a mut self, f: F) -> ListOf<U, S>
    where
        F: FnMut(&'a mut T) -> U,
    {
        ListOf {
            list: self.list.iter_mut().map(f).collect(),
            separator: self.separator.clone(),
        }
    }

    /// Returns self with a new separator, as applied by the given function
    pub fn map_sep<U: Separator, F>(self, f: F) -> ListOf<T, U>
    where
        F: FnOnce(S) -> U,
    {
        ListOf {
            list: self.list,
            separator: f(self.separator),
        }
    }
}

#[test]
fn list_of() {
    assert_eq!(
        ListOf::<i64, Space>::parse_string(""),
        Ok(ListOf {
            list: vec![],
            separator: Space
        })
    );

    assert_eq!(
        ListOf::<i64, Comma>::parse_string(","),
        Err(Error::ExpectedDone)
    );
    assert_eq!(
        ListOf::<i64, Comma>::parse_string("1,"),
        Err(Error::InvalidNumber)
    );
}

#[test]
fn list_of_space() {
    assert_eq!(
        ListOf::<i64, Space>::parse_string("1 2 3"),
        Ok(ListOf {
            list: vec![1, 2, 3],
            separator: Space
        })
    );

    assert_eq!(
        ListOf::<i64, Space>::parse_string("invalid"),
        Err(Error::InvalidNumber)
    );
    assert_eq!(
        ListOf::<i64, Space>::parse_string("1, 2, 3"),
        Err(Error::ExpectedDone)
    );
}

#[test]
fn list_of_space_or_comma() {
    use crate::attribute::core::{Length, Percentage};
    assert_eq!(
        ListOf::<i64, SpaceOrComma>::parse_string("1, 2, 3"),
        Ok(ListOf {
            list: vec![1, 2, 3],
            separator: SpaceOrComma
        })
    );
    assert_eq!(
        ListOf::<i64, SpaceOrComma>::parse_string("1,2,3"),
        Ok(ListOf {
            list: vec![1, 2, 3],
            separator: SpaceOrComma
        })
    );
    assert_eq!(
        ListOf::<Length, SpaceOrComma>::parse_string("23.2350 20.2268px 0.22356em 80.0005%"),
        Ok(ListOf {
            list: vec![
                Length::Length(lightningcss::values::length::LengthValue::Px(23.235)),
                Length::Length(lightningcss::values::length::LengthValue::Px(20.2268)),
                Length::Length(lightningcss::values::length::LengthValue::Em(0.22356)),
                Length::Percentage(Percentage(0.800_005))
            ],
            separator: SpaceOrComma
        })
    );

    assert_eq!(
        ListOf::<i64, SpaceOrComma>::parse_string("1; 2; 3"),
        Err(Error::ExpectedDone)
    );
}

#[test]
fn list_of_semicolon() {
    use crate::attribute::{
        animation::BeginEnd,
        animation_timing::{ClockValue, Metric},
        core::NumberOptionalNumber,
    };
    assert_eq!(
        ListOf::<NumberOptionalNumber, Semicolon>::parse_string("1, 2; 3"),
        Ok(ListOf {
            list: vec![
                NumberOptionalNumber(1.0, Some(2.0)),
                NumberOptionalNumber(3.0, None)
            ],
            separator: Semicolon
        })
    );
    assert_eq!(
        ListOf::<i64, Semicolon>::parse_string("1;2;3"),
        Ok(ListOf {
            list: vec![1, 2, 3],
            separator: Semicolon
        })
    );
    assert_eq!(
        ListOf::<BeginEnd, Semicolon>::parse_string("0;thing2.end"),
        Ok(ListOf {
            list: vec![
                BeginEnd::OffsetValue(ClockValue::TimecountValue {
                    timecount: 0.0,
                    metric: Metric::Second
                }),
                BeginEnd::SyncbaseValue {
                    id: "thing2".into(),
                    begin: false,
                    offset: None
                }
            ],
            separator: Semicolon
        })
    );

    assert_eq!(
        ListOf::<i64, Semicolon>::parse_string("1,2,3"),
        Err(Error::ExpectedDone)
    );
}
