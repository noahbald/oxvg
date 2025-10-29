//! Animation timing attribute types as specified in [animations](https://svgwg.org/specs/animations/#TimingAttributes)
use cssparser_lightningcss::Token;

use crate::{
    enum_attr,
    error::{ParseError, ParseErrorKind, PrinterError},
    parse::{Parse, Parser},
    serialize::{Printer, ToAtom},
};

use super::core::{Integer, Number};

#[derive(Clone, Debug, PartialEq, Eq)]
/// [SMIL](https://www.w3.org/TR/2001/REC-smil-animation-20010904/#Timing-ClockValueSyntax)
pub enum ClockValue {
    /// A time given in hours
    FullClockValue {
        /// Hours
        hours: Integer,
        /// Minutes
        minutes: Integer,
        /// Seconds
        seconds: Integer,
        /// Remainder of seconds (e.g. `10.25` would have a fraction of `25`; 25 milliseconds)
        fraction: Option<Integer>,
    },
    /// A time given in minutes
    PartialClockValue {
        /// Minutes
        minutes: Integer,
        /// Seconds
        seconds: Integer,
        /// Remainder of seconds (e.g. `10.25` would have a fraction of `25`; 25 milliseconds)
        fraction: Option<Integer>,
    },
    /// A time given in some metric
    TimecountValue {
        /// The number of units
        timecount: Integer,
        /// The remainder of units
        fraction: Option<Integer>,
        /// The metric
        metric: Metric,
    },
}
impl<'input> Parse<'input> for ClockValue {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        let digit_location = input.current_source_location();
        let digit = input.expect_integer()?;
        if input.try_parse(Parser::expect_colon).is_err() {
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
        if input.try_parse(Parser::expect_colon).is_err() {
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
                dest.write_char(':')?;
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
                dest.write_char(':')?;
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

#[derive(Clone, Default, Debug, PartialEq, Eq)]
/// The metric used by a timeclock value
pub enum Metric {
    /// Hour
    Hour,
    /// Min
    Min,
    #[default]
    /// Second
    Second,
    /// Milli-second
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

#[derive(Clone, Debug, PartialEq)]
/// Specifies the duration
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#DurAttribute)
/// [w3 | animations](https://svgwg.org/specs/animations/#DurAttribute)
pub enum Dur {
    /// The duration
    ClockValue(ClockValue),
    /// For an element with a defined media, that media's duration
    Media,
    /// Indefinite
    Indefinite,
}
impl<'input> Parse<'input> for Dur {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                let ident: &str = input.expect_ident().map_err(|_| ())?;
                Ok(match ident {
                    "media" => Self::Media,
                    "indefinite" => Self::Indefinite,
                    _ => return Err(()),
                })
            })
            .or_else(|()| ClockValue::parse(input).map(Self::ClockValue))
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

enum_attr!(
    /// How the animation effects the element once complete
    Fill {
        /// The element is frozen at the final value of the animation
        Freeze: "freeze",
        /// The effect of the animation is removed
        Remove: "remove",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// The minimum/maximum duration
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#MinAttribute)
/// [w3 | animations](https://svgwg.org/specs/animations/#MinAttribute)
pub enum MinMax {
    /// The length of the duration
    ClockValue(ClockValue),
    /// For an element with a defined media, that media's duration
    Media,
}
impl<'input> Parse<'input> for MinMax {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| input.expect_ident_matching("media").map(|()| Self::Media))
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
/// Specifies the number of iterations for the animation
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#RepeatCountAttribute)
/// [w3 | animations](https://svgwg.org/specs/animations/#RepeatCountAttribute)
pub enum RepeatCount {
    /// A number greater than zero specifying the number of iterations
    Number(Number),
    /// The animation will repeat indefinitely
    Indefinite,
}
impl<'input> Parse<'input> for RepeatCount {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let result = input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|()| Self::Indefinite)
            })
            .or_else(|_| Number::parse(input).map(Self::Number))?;
        if let Self::Number(number) = result {
            if number <= 0.0 {
                return Err(input.new_custom_error(ParseErrorKind::CSSParserError(
                    lightningcss::error::ParserError::InvalidValue,
                )));
            }
        }
        Ok(result)
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
/// Specifies the duration for a repeat
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#RepeatDurAttribute)
/// [w3 | animations](https://svgwg.org/specs/animations/#RepeatDurAttribute)
pub enum RepeatDur {
    /// The duration to repeat the the animation
    ClockValue(ClockValue),
    /// The animation repeats indefinitely
    Indefinite,
}
impl<'input> Parse<'input> for RepeatDur {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|()| Self::Indefinite)
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

enum_attr!(
    /// When an animation can be restarted
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#RestartAttribute)
    /// [w3 | animations](https://svgwg.org/specs/animations/#RestartAttribute)
    Restart {
        /// The animation can be restarted anytime
        Always: "always",
        /// The animation can only be restarted when it's not active
        WhenNotActive: "whenNotActive",
        /// The animation can never be restarted
        Never: "never",
    }
);
