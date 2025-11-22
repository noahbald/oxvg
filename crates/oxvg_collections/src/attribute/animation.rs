//! Animation attribute types as specified in [animations](https://svgwg.org/specs/animations/)
#[cfg(feature = "parse")]
use oxvg_parse::{error::Error, Parse, Parser};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};

use crate::{atom::Atom, enum_attr};

use super::{
    animation_timing::ClockValue,
    core::{Integer, Number},
};

enum_attr!(
    /// Specifies the namespace in which the target attribute and its associated values are defined.
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#AttributeTypeAttribute)
    AttributeType {
        /// This specifies that the value of ‘attributeName’ is the name of a CSS property.
        CSS: "CSS",
        /// This specifies that the value of ‘attributeName’ is the name of an XML attribute.
        XML: "XML",
        /// he implementation should match the ‘attributeName’ to an attribute for the target element.
        Auto: "auto",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// Defines when the element should begin
/// [w3](https://svgwg.org/specs/animations/#BeginValueListSyntax)
pub enum BeginEnd<'i> {
    /// (Clock-value)
    OffsetValue(ClockValue),
    /// (Id-value "." ( "begin" | "end" )) (Clock-value)?
    SyncbaseValue {
        /// An ID reference to another element that has animations to sync with
        id: Atom<'i>,
        /// Whether the animation should sync with the beginning or end of the referenced element
        begin: bool,
        /// The clock time to delay the synced animation by
        offset: Option<ClockValue>,
    },
    /// (Id-value ".")? (Event-ref) (Clock-value)?
    EventValue {
        /// An ID reference to another element that has events to sync with
        id: Option<Atom<'i>>,
        // TODO: Event ID
        /// The event name to sync animations with
        event: Atom<'i>,
        /// The clock time to delay the synced animation by
        offset: Option<ClockValue>,
    },
    /// (Id-value ".")? "repeat(<integer>)" (Clock-value)?
    RepeatValue {
        /// An ID reference to another element
        id: Option<Atom<'i>>,
        /// The number of repetitions
        repeat: Integer,
        /// The clock time to delay the synced animation by
        offset: Option<ClockValue>,
    },
    /// "accessKey(<character>)" (Clock-value)?
    AccessKeyValue {
        /// The key name that will begin the animation when pressed by the user
        character: Atom<'i>,
        /// The clock time to delay the synced animation by
        offset: Option<ClockValue>,
    },
    /// "wallclock(<wallclock-value>)"
    WallclockSyncValue(Atom<'i>),
    /// "indefinite"
    Indefinite,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for BeginEnd<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|()| Self::Indefinite)
            })
            .or_else(|_| input.try_parse(ClockValue::parse).map(Self::OffsetValue))
            .or_else(|_| {
                input.try_parse(|input| {
                    input.expect_ident_matching("accessKey")?;
                    input.expect_char('(')?;
                    let character = input.take_matches(|char| char != ')').into();
                    input.expect_char(')')?;
                    let offset = input.try_parse(ClockValue::parse).ok();
                    Ok(Self::AccessKeyValue { character, offset })
                })
            })
            .or_else(|_: Error<'input>| {
                input.try_parse(|input| {
                    input.expect_ident_matching("wallclock")?;
                    input.expect_char('(')?;
                    let wallclock_value = input.take_matches(|char| char != ')').into();
                    input.expect_char(')')?;
                    Ok(Self::WallclockSyncValue(wallclock_value))
                })
            })
            .or_else(|_: Error<'input>| {
                let id = input.expect_ident()?;
                let id = match id.rfind(['-', '+']) {
                    Some(n) => {
                        input.rewind(id.len() - n);
                        &id[..n]
                    }
                    None => id,
                };
                let (id, event) = match id.rsplit_once('.') {
                    Some((id, event)) => (Some(id), event),
                    None => (None, id),
                };
                if event == "repeat" {
                    input.expect_char('(')?;
                    let repeat = i32::parse(input)?;
                    input.expect_char(')')?;
                    let offset = ClockValue::parse(input).ok();
                    return Ok(Self::RepeatValue {
                        id: id.map(Into::into),
                        repeat,
                        offset,
                    });
                }
                input.skip_whitespace();
                let offset = input.try_parse(ClockValue::parse).ok();
                if let (Some(id), "begin" | "end") = (id, event) {
                    Ok(Self::SyncbaseValue {
                        id: id.into(),
                        begin: event == "begin",
                        offset,
                    })
                } else {
                    Ok(Self::EventValue {
                        id: id.map(Into::into),
                        event: event.into(),
                        offset,
                    })
                }
            })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for BeginEnd<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::OffsetValue(clock_value) => clock_value.write_value(dest),
            Self::SyncbaseValue { id, begin, offset } => {
                dest.write_str(id)?;
                dest.write_char('.')?;
                if *begin {
                    dest.write_str("begin")?;
                } else {
                    dest.write_str("end")?;
                }
                if let Some(clock_value) = offset {
                    if !clock_value.is_negative() {
                        dest.write_char('+')?;
                    }
                    clock_value.write_value(dest)?;
                }
                Ok(())
            }
            Self::EventValue { id, event, offset } => {
                if let Some(id) = id {
                    dest.write_str(id)?;
                    dest.write_char('.')?;
                }
                dest.write_str(event)?;
                if let Some(clock_value) = offset {
                    if !clock_value.is_negative() {
                        dest.write_char('+')?;
                    }
                    clock_value.write_value(dest)?;
                }
                Ok(())
            }
            Self::RepeatValue { id, repeat, offset } => {
                if let Some(id) = id {
                    dest.write_str(id)?;
                    dest.write_char('.')?;
                }
                dest.write_str("repeat(")?;
                repeat.write_value(dest)?;
                dest.write_char(')')?;
                if let Some(clock_value) = offset {
                    if !clock_value.is_negative() {
                        dest.write_char('+')?;
                    }
                    clock_value.write_value(dest)?;
                }
                Ok(())
            }
            Self::AccessKeyValue { character, offset } => {
                dest.write_str("accessKey(")?;
                dest.write_str(character)?;
                dest.write_char(')')?;
                if let Some(clock_value) = offset {
                    if !clock_value.is_negative() {
                        dest.write_char('+')?;
                    }
                    clock_value.write_value(dest)?;
                }
                Ok(())
            }
            Self::WallclockSyncValue(wallclock_value) => {
                dest.write_str("wallclock(")?;
                dest.write_str(wallclock_value)?;
                dest.write_char(')')
            }
            Self::Indefinite => dest.write_str("indefinite"),
        }
    }
}
#[test]
fn begin_end() {
    use crate::attribute::animation_timing::Metric;
    let clock_value = ClockValue::TimecountValue {
        timecount: -15.0,
        metric: Metric::Second,
    };

    assert_eq!(
        BeginEnd::parse_string("0"),
        Ok(BeginEnd::OffsetValue(ClockValue::TimecountValue {
            timecount: 0.0,
            metric: Metric::Second
        }))
    );
    assert_eq!(
        BeginEnd::parse_string("-15s"),
        Ok(BeginEnd::OffsetValue(clock_value.clone()))
    );
    assert_eq!(
        BeginEnd::parse_string("id.begin -15s"),
        Ok(BeginEnd::SyncbaseValue {
            id: "id".into(),
            begin: true,
            offset: Some(clock_value.clone())
        })
    );
    assert_eq!(
        BeginEnd::parse_string("id.end"),
        Ok(BeginEnd::SyncbaseValue {
            id: "id".into(),
            begin: false,
            offset: None
        })
    );
    assert_eq!(
        BeginEnd::parse_string("id2.end"),
        Ok(BeginEnd::SyncbaseValue {
            id: "id2".into(),
            begin: false,
            offset: None
        })
    );
    assert_eq!(
        BeginEnd::parse_string("onclick-15s"),
        Ok(BeginEnd::EventValue {
            id: None,
            event: "onclick".into(),
            offset: Some(clock_value.clone())
        })
    );
    assert_eq!(
        BeginEnd::parse_string("id.onclick"),
        Ok(BeginEnd::EventValue {
            id: Some("id".into()),
            event: "onclick".into(),
            offset: None
        })
    );
    assert_eq!(
        BeginEnd::parse_string("repeat(1) -15s"),
        Ok(BeginEnd::RepeatValue {
            id: None,
            repeat: 1,
            offset: Some(clock_value.clone())
        })
    );
    assert_eq!(
        BeginEnd::parse_string("id.repeat(0)"),
        Ok(BeginEnd::RepeatValue {
            id: Some("id".into()),
            repeat: 0,
            offset: None
        })
    );
    assert_eq!(
        BeginEnd::parse_string("accessKey(s)-15s"),
        Ok(BeginEnd::AccessKeyValue {
            character: "s".into(),
            offset: Some(clock_value.clone())
        })
    );
    assert_eq!(
        BeginEnd::parse_string("wallclock(01/01/1960)"),
        Ok(BeginEnd::WallclockSyncValue("01/01/1960".into()))
    );
    assert_eq!(
        BeginEnd::parse_string("indefinite"),
        Ok(BeginEnd::Indefinite)
    );

    assert_eq!(BeginEnd::parse_string("0;"), Err(Error::ExpectedDone));
}

enum_attr!(
    /// Specifies the interpolation mode for the animation.
    /// [w3](https://svgwg.org/specs/animations/#CalcModeAttribute)
    CalcMode {
        /// This specifies that the animation function will jump from one value to the next without any interpolation.
        Discrete: "discrete",
        /// Simple linear interpolation between values is used to calculate the animation function.
        Linear: "linear",
        /// Defines interpolation to produce an even pace of change across the animation.
        Paced: "paced",
        /// Interpolates from one value in the 'values' list to the next according to a time function defined by a cubic Bézier spline.
        Spline: "spline",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// A set of Bézier control points associated with the ‘keyTimes’ list
/// [w3](https://svgwg.org/specs/animations/#KeySplinesAttribute)
pub struct ControlPoint(pub [Number; 4]);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for ControlPoint {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input.skip_whitespace();
        let x1 = Number::parse(input)?;
        input.skip_whitespace();
        input.skip_char(',');
        input.skip_whitespace();
        let y1 = Number::parse(input)?;
        input.skip_whitespace();
        input.skip_char(',');
        input.skip_whitespace();
        let x2 = Number::parse(input)?;
        input.skip_whitespace();
        input.skip_char(',');
        input.skip_whitespace();
        let y2 = Number::parse(input)?;
        Ok(Self([x1, y1, x2, y2]))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for ControlPoint {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let Self([x1, y1, x2, y2]) = self;
        x1.write_value(dest)?;
        dest.write_char(' ')?;
        y1.write_value(dest)?;
        dest.write_char(' ')?;
        x2.write_value(dest)?;
        dest.write_char(' ')?;
        y2.write_value(dest)
    }
}
