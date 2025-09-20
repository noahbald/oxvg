use std::ops::Deref;

use cssparser_lightningcss::Token;
use lightningcss::{
    declaration::DeclarationBlock,
    properties::svg::SVGPaint,
    stylesheet::ParserOptions,
    values::{
        color::CssColor,
        length::LengthValue,
        number::{CSSInteger, CSSNumber},
    },
};

pub use lightningcss::values::{percentage::Percentage, time::Time};

use crate::{
    atom::Atom,
    error::{ParseError, ParseErrorKind, PrinterError},
    parse::{Parse, Parser},
    serialize::{Printer, ToAtom},
};

use super::transform::SVGTransformList;

pub type Angle = lightningcss::values::angle::Angle;
pub type Anything<'i> = Atom<'i>;

#[derive(Debug, Clone, PartialEq)]
pub struct Boolean<'input>(Option<Atom<'input>>);
impl<'input> Parse<'input> for Boolean<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        Ok(Self(
            input
                .try_parse(|input| -> Result<Atom<'input>, ()> {
                    let ident = input.expect_ident().map_err(|_| ())?;
                    Ok(Atom::Cow(ident.into()))
                })
                .ok(),
        ))
    }
}
impl<'input> ToAtom for Boolean<'input> {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match &self.0 {
            Some(name) => name.write_atom(dest),
            None => Ok(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClockValue {
    FullClockValue {
        hours: Integer,
        minutes: Integer,
        seconds: Integer,
        fraction: Option<Integer>,
    },
    PartialClockValue {
        minutes: Integer,
        seconds: Integer,
        fraction: Option<Integer>,
    },
    TimecountValue {
        timecount: Integer,
        fraction: Option<Integer>,
        metric: Metric,
    },
}
impl<'input> Parse<'input> for ClockValue {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        let digit_location = input.current_source_location();
        let digit = input.expect_integer()?;
        if input.try_parse(|input| input.expect_colon()).is_err() {
            // Timecount value
            let timecount = digit;
            let fraction = input
                .try_parse(|input| {
                    input.expect_ident_matching(".")?;
                    input.expect_integer()
                })
                .ok();
            let metric = input.try_parse(Metric::parse).unwrap_or_default();
            return Ok(Self::TimecountValue {
                timecount,
                fraction,
                metric,
            });
        }
        input.expect_colon()?;
        let second_digit_location = input.current_source_location();
        let second_digit = input.expect_integer()?;
        if input.try_parse(|input| input.expect_colon()).is_err() {
            // Partial clock
            let minutes = digit;
            if !(0..60).contains(&minutes) {
                return Err(digit_location.new_custom_error(ParseErrorKind::InvalidClockValue));
            }
            let seconds = second_digit;
            if !(0..60).contains(&seconds) {
                return Err(
                    second_digit_location.new_custom_error(ParseErrorKind::InvalidClockValue)
                );
            }
            let fraction = input
                .try_parse(|input| {
                    input.expect_ident_matching(".")?;
                    input.expect_integer()
                })
                .ok();
            return Ok(Self::PartialClockValue {
                minutes,
                seconds,
                fraction,
            });
        }
        // Full clock
        input.expect_colon()?;
        let hours = digit;
        let minutes = second_digit;
        if !(0..60).contains(&minutes) {
            return Err(second_digit_location.new_custom_error(ParseErrorKind::InvalidClockValue));
        }
        let seconds_location = input.current_source_location();
        let seconds = input.expect_integer()?;
        if !(0..60).contains(&seconds) {
            return Err(seconds_location.new_custom_error(ParseErrorKind::InvalidClockValue));
        }
        let fraction = input
            .try_parse(|input| {
                input.expect_ident_matching(".")?;
                input.expect_integer()
            })
            .ok();
        Ok(Self::FullClockValue {
            hours,
            minutes,
            seconds,
            fraction,
        })
    }
}
impl ToAtom for ClockValue {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::TimecountValue {
                timecount,
                fraction,
                metric,
            } => {
                timecount.write_atom(dest)?;
                if let Some(fraction) = fraction {
                    dest.write_char('.')?;
                    fraction.write_atom(dest)?;
                }
                metric.write_atom(dest)
            }
            Self::PartialClockValue {
                minutes,
                seconds,
                fraction,
            } => {
                minutes.write_atom(dest)?;
                dest.write_char(':');
                seconds.write_atom(dest)?;
                if let Some(fraction) = fraction {
                    dest.write_char('.')?;
                    fraction.write_atom(dest)?;
                }
                Ok(())
            }
            Self::FullClockValue {
                hours,
                minutes,
                seconds,
                fraction,
            } => {
                hours.write_atom(dest)?;
                dest.write_char(':')?;
                minutes.write_atom(dest)?;
                dest.write_char(':');
                seconds.write_atom(dest)?;
                if let Some(fraction) = fraction {
                    dest.write_char('.')?;
                    fraction.write_atom(dest)?;
                }
                Ok(())
            }
        }
    }
}

pub type Color = CssColor;
pub type Coordinate = Length;

#[derive(Clone, Debug, PartialEq)]
pub enum Length {
    Number(Number),
    Length(LengthValue),
    Percentage(Percentage),
}
impl<'input> Parse<'input> for Length {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(Percentage::parse)
            .map(Self::Percentage)
            .or_else(|_| input.try_parse(LengthValue::parse).map(Self::Length))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
impl ToAtom for Length {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_atom(dest),
            Self::Length(length) => length.write_atom(dest),
            Self::Percentage(percentage) => percentage.write_atom(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Frequency {
    Hz(Number),
    KHz(Number),
}
impl<'input> Parse<'input> for Frequency {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let number = Number::parse(input)?;
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        let str: &str = &*ident;
        Ok(match str {
            "Hz" => Self::Hz(number),
            "KHz" => Self::KHz(number),
            _ => return Err(location.new_unexpected_token_error(Token::Ident(ident.clone()))),
        })
    }
}
impl ToAtom for Frequency {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Hz(number) => {
                number.write_atom(dest)?;
                dest.write_str("Hz")
            }
            Self::KHz(number) => {
                number.write_atom(dest)?;
                dest.write_str("KHz")
            }
        }
    }
}

pub type FuncIRI<'i> = Atom<'i>;
pub type Integer = CSSInteger;
pub type IRI<'i> = Atom<'i>;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum Metric {
    Hour,
    Min,
    #[default]
    Second,
    MilliSecond,
}
impl<'input> Parse<'input> for Metric {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        Ok(match std::ops::Deref::deref(ident) {
            "h" => Self::Hour,
            "min" => Self::Min,
            "s" => Self::Second,
            "ms" => Self::MilliSecond,
            _ => return Err(location.new_unexpected_token_error(Token::Ident(ident.clone()))),
        })
    }
}
impl ToAtom for Metric {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Hour => dest.write_char('h'),
            Self::Min => dest.write_str("min"),
            Self::Second => Ok(()), // default can be omitted
            Self::MilliSecond => dest.write_str("ms"),
        }
    }
}

pub type Name<'i> = Atom<'i>;
pub type Number = CSSNumber;

#[derive(Clone, Debug, PartialEq)]
pub struct NumberOptionalNumber(Number, Option<Number>);
impl<'input> Parse<'input> for NumberOptionalNumber {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let a = Number::parse(input)?;
        input.skip_whitespace();
        let has_comma = input.try_parse(Parser::expect_comma).is_ok();
        input.skip_whitespace();
        let b = if has_comma {
            Some(Number::parse(input)?)
        } else {
            input.try_parse(Number::parse).ok()
        };
        Ok(Self(a, b))
    }
}
impl ToAtom for NumberOptionalNumber {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_atom(dest)?;
        if let Some(b) = self.1 {
            dest.write_char(' ')?;
            b.write_atom(dest)?;
        }
        Ok(())
    }
}

pub type Opacity = Number;
pub type Paint<'i> = SVGPaint<'i>;

#[derive(Clone, Debug, PartialEq)]
pub struct Style<'i>(DeclarationBlock<'i>);
impl<'input> Parse<'input> for Style<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        DeclarationBlock::parse(input, &ParserOptions::default())
            .map(Self)
            .map_err(ParseErrorKind::from_css)
    }
}
impl ToAtom for Style<'_> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_atom(dest)
    }
}
impl<'input> Deref for Style<'input> {
    type Target = DeclarationBlock<'input>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type TransformList = SVGTransformList;
pub type Url<'i> = Atom<'i>;
