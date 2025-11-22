//! Animation timing attribute types as specified in [animations](https://svgwg.org/specs/animations/#TimingAttributes)
use lightningcss::values::number::CSSNumber;

#[cfg(feature = "parse")]
use oxvg_parse::{error::Error, Parse, Parser};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};

use crate::enum_attr;

use super::core::{Integer, Number};

#[derive(Clone, Debug, PartialEq)]
/// [SMIL](https://www.w3.org/TR/2001/REC-smil-animation-20010904/#Timing-ClockValueSyntax)
pub enum ClockValue {
    /// A time given in hours
    FullClockValue {
        /// Hours
        hours: Integer,
        /// Minutes
        minutes: Integer,
        /// Seconds
        seconds: CSSNumber,
    },
    /// A time given in minutes
    PartialClockValue {
        /// Minutes
        minutes: Integer,
        /// Seconds
        seconds: CSSNumber,
    },
    /// A time given in some metric
    TimecountValue {
        /// The number of units
        timecount: CSSNumber,
        /// The metric
        metric: Metric,
    },
}
impl ClockValue {
    /// Returns whether the signed clock-value is negative
    pub fn is_negative(&self) -> bool {
        match self {
            Self::FullClockValue { hours, .. } => hours.is_negative(),
            Self::PartialClockValue { minutes, .. } => minutes.is_negative(),
            Self::TimecountValue { timecount, .. } => timecount.is_sign_negative(),
        }
    }
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for ClockValue {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        // NOTE: Technically clock-value isn't allowed sign (+/-), but we allow it here for easier parsing
        // where clock-value *is* used *and* allows signs
        let timecount = f32::parse(input)?;
        Ok(input
            .try_parse(|input| {
                Ok(Self::TimecountValue {
                    timecount,
                    metric: Metric::parse_string(input.expect_ident()?)?,
                })
            })
            .or_else(|_: Error<'input>| {
                input.try_parse(|input| {
                    let timecount = timecount as Integer;
                    input.expect_char(':')?;
                    let minutes_or_seconds = f32::parse(input)?;
                    if input.try_parse(|input| input.expect_char(':')).is_err() {
                        return Ok(Self::PartialClockValue {
                            minutes: timecount,
                            seconds: minutes_or_seconds,
                        });
                    }
                    if minutes_or_seconds.trunc() != minutes_or_seconds {
                        return Err(Error::InvalidNumber);
                    }
                    let minutes = minutes_or_seconds as Integer;
                    let seconds = f32::parse(input)?;
                    Ok(Self::FullClockValue {
                        hours: timecount,
                        minutes,
                        seconds,
                    })
                })
            })
            .unwrap_or(Self::TimecountValue {
                timecount,
                metric: Metric::Second,
            }))
    }
}

#[cfg(feature = "serialize")]
impl ToValue for ClockValue {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::TimecountValue { timecount, metric } => {
                timecount.write_value(dest)?;
                if !matches!(metric, Metric::Second) {
                    metric.write_value(dest)?;
                }
                Ok(())
            }
            Self::PartialClockValue { minutes, seconds } => {
                if *minutes < 10 {
                    dest.write_char('0')?;
                }
                minutes.write_value(dest)?;
                dest.write_char(':')?;
                if *seconds < 10.0 {
                    dest.write_char('0')?;
                }
                seconds.write_value(dest)?;
                Ok(())
            }
            Self::FullClockValue {
                hours,
                minutes,
                seconds,
            } => {
                if *hours < 10 {
                    dest.write_char('0')?;
                }
                hours.write_value(dest)?;
                dest.write_char(':')?;
                if *minutes < 10 {
                    dest.write_char('0')?;
                }
                minutes.write_value(dest)?;
                dest.write_char(':')?;
                if *seconds < 10.0 {
                    dest.write_char('0')?;
                }
                seconds.write_value(dest)?;
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
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Metric {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        Ok(input
            .try_parse(|input| {
                let ident = input.expect_ident().map_err(|_| ())?;
                Ok(match ident {
                    "h" => Self::Hour,
                    "min" => Self::Min,
                    "s" => Self::Second,
                    "ms" => Self::MilliSecond,
                    _ => return Err(()),
                })
            })
            .unwrap_or(Self::Second))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Metric {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
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
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Dur {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
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
#[cfg(feature = "serialize")]
impl ToValue for Dur {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::ClockValue(clock_value) => clock_value.write_value(dest),
            Self::Media => dest.write_str("media"),
            Self::Indefinite => dest.write_str("indefinite"),
        }
    }
}

enum_attr!(
    /// How the animation effects the element once complete
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#FillAttribute)
    /// [w3 | SVG 2](https://svgwg.org/specs/animations/#FillAttribute)
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
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for MinMax {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| input.expect_ident_matching("media").map(|()| Self::Media))
            .or_else(|_| Ok(Self::ClockValue(ClockValue::parse(input)?)))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for MinMax {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::ClockValue(clock_value) => clock_value.write_value(dest),
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
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for RepeatCount {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let result = input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|()| Self::Indefinite)
            })
            .or_else(|_| Number::parse(input).map(Self::Number))?;
        if let Self::Number(number) = result {
            if number <= 0.0 {
                return Err(Error::InvalidRange);
            }
        }
        Ok(result)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for RepeatCount {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_value(dest),
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
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for RepeatDur {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|()| Self::Indefinite)
            })
            .or_else(|_| Ok(Self::ClockValue(ClockValue::parse(input)?)))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for RepeatDur {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::ClockValue(clock_value) => clock_value.write_value(dest),
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

#[test]
fn clock_value() {
    use oxvg_serialize::PrinterOptions;

    assert_eq!(
        ClockValue::parse_string("00:30:03")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("00:30:03")
    );
    assert_eq!(
        ClockValue::parse_string("50:00:10.25")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("50:00:10.25")
    );
    assert_eq!(
        ClockValue::parse_string("02:33")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("02:33")
    );
    assert_eq!(
        ClockValue::parse_string("00:10.5")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("00:10.5")
    );
    assert_eq!(
        ClockValue::parse_string("-3.2h")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("-3.2h")
    );
    assert_eq!(
        ClockValue::parse_string("+45min")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("45min")
    );
    assert_eq!(
        ClockValue::parse_string("30s")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("30")
    );
    assert_eq!(
        ClockValue::parse_string("5ms")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("5ms")
    );
    assert_eq!(
        ClockValue::parse_string("12.467")
            .unwrap()
            .to_value_string(PrinterOptions::default())
            .unwrap(),
        String::from("12.467")
    );

    assert_eq!(ClockValue::parse_string("0;"), Err(Error::ExpectedDone));
}
