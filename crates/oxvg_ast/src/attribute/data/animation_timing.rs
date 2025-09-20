use crate::{
    enum_attr,
    error::{ParseErrorKind, PrinterError},
    parse::Parse,
    serialize::{Printer, ToAtom},
};

use super::core::{ClockValue, Number};

#[derive(Clone, Debug, PartialEq)]
pub enum Dur {
    ClockValue(ClockValue),
    Media,
    Indefinite,
}
impl<'input> Parse<'input> for Dur {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                Ok(match ident {
                    "media" => Self::Media,
                    "indefinite" => Self::Indefinite,
                    _ => return Err(()),
                })
            })
            .or_else(|_| ClockValue::parse(input).map(Self::ClockValue))
    }
}
impl ToAtom for Dur {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::ClockValue(clock_value) => clock_value.write_atom(dest),
            Self::Media => dest.write_str("media"),
            Self::Indefinite => dest.write_str("indefinite"),
        }
    }
}

enum_attr!(Fill {
    Freeze: "freeze",
    Remove: "remove",
});

#[derive(Clone, Debug, PartialEq)]
pub enum MinMax {
    ClockValue(ClockValue),
    Media,
}
impl<'input> Parse<'input> for MinMax {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| input.expect_ident_matching("media").map(|_| Self::Media))
            .or_else(|_| Ok(Self::ClockValue(ClockValue::parse(input)?)))
    }
}
impl ToAtom for MinMax {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::ClockValue(clock_value) => clock_value.write_atom(dest),
            Self::Media => dest.write_str("media"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RepeatCount {
    Number(Number),
    Indefinite,
}
impl<'input> Parse<'input> for RepeatCount {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|_| Self::Indefinite)
            })
            .or_else(|_| Ok(Self::Number(Number::parse(input)?)))
    }
}
impl ToAtom for RepeatCount {
    fn write_atom<W>(&self, dest: &mut crate::serialize::Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_atom(dest),
            Self::Indefinite => dest.write_str("indefinite"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RepeatDur {
    ClockValue(ClockValue),
    Indefinite,
}
impl<'input> Parse<'input> for RepeatDur {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|_| Self::Indefinite)
            })
            .or_else(|_| Ok(Self::ClockValue(ClockValue::parse(input)?)))
    }
}
impl ToAtom for RepeatDur {
    fn write_atom<W>(&self, dest: &mut crate::serialize::Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::ClockValue(clock_value) => clock_value.write_atom(dest),
            Self::Indefinite => dest.write_str("indefinite"),
        }
    }
}

enum_attr!(Restart {
    Always: "always",
    WhenNotActive: "whenNotActive",
    Never: "never",
});
