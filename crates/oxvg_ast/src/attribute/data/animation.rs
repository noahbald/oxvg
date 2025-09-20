use crate::{
    atom::Atom,
    enum_attr,
    error::{ParseError, ParseErrorKind, PrinterError},
    parse::{Parse, Parser},
    serialize::{Printer, ToAtom},
};

use super::core::{ClockValue, Integer, Number};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BeginEnd<'i> {
    /// (Clock-value)
    OffsetValue(ClockValue),
    /// (Id-value "." ( "begin" | "end" )) (Clock-value)?
    SyncbaseValue {
        id: Atom<'i>,
        begin: bool,
        offset: Option<ClockValue>,
    },
    /// (Id-value ".")? (Event-ref) (Clock-value)?
    EventValue {
        id: Option<Atom<'i>>,
        // TODO: Event ID
        event: Atom<'i>,
        offset: Option<ClockValue>,
    },
    /// (Id-value ".")? "repeat(<integer>)" (Clock-value)?
    RepeatValue {
        id: Option<Atom<'i>>,
        repeat: Integer,
        offset: Option<ClockValue>,
    },
    /// "accessKey(<character>)" (Clock-value)?
    AccessKeyValue {
        character: Atom<'i>,
        offset: Option<ClockValue>,
    },
    /// "wallclock(<wallclock-value>)"
    WallclockSyncValue(Atom<'i>),
    /// "indefinite"
    Indefinite,
}
impl<'input> Parse<'input> for BeginEnd<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("indefinite")
                    .map(|_| Self::Indefinite)
            })
            .or_else(|_| input.try_parse(ClockValue::parse).map(Self::OffsetValue))
            .or_else(|_| {
                input.try_parse(|input| {
                    input
                        .expect_function_matching("accessKey")
                        .map_err(ParseErrorKind::from_basic)?;
                    let character = input.parse_nested_block(|input| {
                        let ident = input.expect_ident().map_err(ParseErrorKind::from_basic)?;
                        Ok(Atom::Cow(ident.into()))
                    })?;
                    let offset = input.try_parse(ClockValue::parse).ok();
                    let result: Result<Self, ParseError<'input>> =
                        Ok(Self::AccessKeyValue { character, offset });
                    result
                })
            })
            .or_else(|_| {
                input.try_parse(|input| {
                    input.expect_function_matching("wallclock")?;
                    let wallclock_value = input
                        .parse_nested_block(|input| Ok(input.slice_from(input.position())))?
                        .into();
                    let result: Result<Self, ParseError<'input>> =
                        Ok(Self::WallclockSyncValue(wallclock_value));
                    result
                })
            })
            .or_else(|_| {
                let start = input.current_source_location();
                let id = input
                    .try_parse(|input| {
                        let id_value = input.expect_ident()?.into();
                        input.expect_delim('.')?;
                        let result: Result<_, cssparser_lightningcss::BasicParseError<'input>> =
                            Ok(Atom::Cow(id_value));
                        result
                    })
                    .ok();
                if let Ok(_) = input.try_parse(|input| input.expect_function_matching("repeat")) {
                    let repeat = input.parse_nested_block(Integer::parse)?;
                    let offset = input.try_parse(ClockValue::parse).ok();
                    return Ok(Self::RepeatValue { id, repeat, offset });
                } else if let Ok(begin) = input.try_parse(|input| {
                    let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                    Ok(match ident {
                        "begin" => true,
                        "end" => false,
                        _ => return Err(()),
                    })
                }) {
                    let Some(id) = id else {
                        return Err(start.new_custom_error(ParseErrorKind::MissingSyncbaseId));
                    };
                    let offset = input.try_parse(ClockValue::parse).ok();
                    Ok(Self::SyncbaseValue { id, begin, offset })
                } else {
                    let event = Atom::Cow(input.expect_ident()?.into());
                    let offset = input.try_parse(ClockValue::parse).ok();
                    Ok(Self::EventValue { id, event, offset })
                }
            })
    }
}
impl<'input> ToAtom for BeginEnd<'input> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::OffsetValue(clock_value) => clock_value.write_atom(dest),
            Self::SyncbaseValue { id, begin, offset } => {
                dest.write_str(id)?;
                dest.write_char('.')?;
                if *begin {
                    dest.write_str("begin")?;
                } else {
                    dest.write_str("end")?;
                }
                if let Some(clock_value) = offset {
                    clock_value.write_atom(dest)?;
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
                    clock_value.write_atom(dest)?;
                }
                Ok(())
            }
            Self::RepeatValue { id, repeat, offset } => {
                if let Some(id) = id {
                    dest.write_str(id)?;
                    dest.write_char('.')?;
                }
                dest.write_str("repeat(")?;
                repeat.write_atom(dest)?;
                dest.write_char(')')?;
                if let Some(clock_value) = offset {
                    clock_value.write_atom(dest)?;
                }
                Ok(())
            }
            Self::AccessKeyValue { character, offset } => {
                dest.write_str("accessKey(")?;
                dest.write_str(character)?;
                dest.write_char(')')?;
                if let Some(clock_value) = offset {
                    clock_value.write_atom(dest)?;
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

enum_attr!(CalcMode {
    Discrete: "discrete",
    Linear: "linear",
    Paced: "paced",
    Spline: "spline",
});

#[derive(Clone, Debug, PartialEq)]
pub struct ControlPoint([Number; 4]);
impl<'input> Parse<'input> for ControlPoint {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input.skip_whitespace();
        let x1 = Number::parse(input)?;
        input.skip_whitespace();
        input.try_parse(Parser::expect_comma);
        input.skip_whitespace();
        let y1 = Number::parse(input)?;
        input.skip_whitespace();
        input.try_parse(Parser::expect_comma);
        input.skip_whitespace();
        let x2 = Number::parse(input)?;
        input.skip_whitespace();
        input.try_parse(Parser::expect_comma);
        input.skip_whitespace();
        let y2 = Number::parse(input)?;
        Ok(Self([x1, y1, x2, y2]))
    }
}
impl ToAtom for ControlPoint {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let Self([x1, y1, x2, y2]) = self;
        x1.write_atom(dest)?;
        dest.write_char(' ')?;
        y1.write_atom(dest)?;
        dest.write_char(' ')?;
        x2.write_atom(dest)?;
        dest.write_char(' ')?;
        y2.write_atom(dest)
    }
}
