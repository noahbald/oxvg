//! Types used for parsing a string of path data.
use crate::{command, Path};

pub use oxvg_parse::{error::PathError, Parse};

/// An error that can occur while parsing path data
pub type Error = PathError;

impl<'input> Parse<'input> for Path {
    fn parse(
        input: &mut oxvg_parse::Parser<'input>,
    ) -> Result<Self, oxvg_parse::error::Error<'input>> {
        let mut result = Path(vec![]);
        result.parse_extend(input, false).map_err(|err| err.error)?;
        Ok(result)
    }
}
impl<'input> Path {
    /// Extends an existing path by reading through a parser input
    ///
    /// # Errors
    ///
    /// If parsing fails
    pub fn parse_extend(
        &mut self,
        input: &mut oxvg_parse::Parser<'input>,
        allow_implicit_start: bool,
    ) -> Result<(), oxvg_parse::error::ParseError<'input>> {
        let list = &mut self.0;

        while !input.is_empty() {
            input.skip_whitespace();
            if !list.is_empty() {
                input.skip_char(',');
                input.skip_whitespace();
            }
            let mut command_id = input
                .try_parse(command::ID::parse)
                .or_else(|_| {
                    if let Some(last) = list.last() {
                        Ok(command::ID::Implicit(Box::new(last.id().next_implicit())))
                    } else if allow_implicit_start {
                        Ok(command::ID::MoveTo)
                    } else {
                        Err(oxvg_parse::error::Error::Path(PathError::NoCommand))
                    }
                })
                .map_err(|error| oxvg_parse::error::ParseError {
                    error,
                    remaining_content: input.take_slice(),
                })?;
            if let Some(last) = list.last() {
                if !command_id.is_implicit() && last.id().next_implicit() == command_id {
                    command_id = command::ID::Implicit(Box::new(command_id));
                }
            } else if !matches!(command_id, command::ID::MoveBy | command::ID::MoveTo) {
                return Err(oxvg_parse::error::ParseError {
                    error: oxvg_parse::error::Error::ExpectedIdent {
                        expected: "implicit or `m` or `M`",
                        received: "other",
                    },
                    remaining_content: input.take_slice(),
                });
            }

            list.push(
                input
                    .try_parse(|input| command::Data::parse(input, command_id))
                    .map_err(|error| oxvg_parse::error::ParseError {
                        error,
                        remaining_content: input.take_slice(),
                    })?,
            );
        }
        Ok(())
    }
}

impl command::ID {
    fn parse<'input>(
        input: &mut oxvg_parse::Parser<'input>,
    ) -> Result<Self, oxvg_parse::error::Error<'input>> {
        match input.read()? {
            'M' => Ok(Self::MoveTo),
            'm' => Ok(Self::MoveBy),
            'L' => Ok(Self::LineTo),
            'l' => Ok(Self::LineBy),
            'H' => Ok(Self::HorizontalLineTo),
            'h' => Ok(Self::HorizontalLineBy),
            'V' => Ok(Self::VerticalLineTo),
            'v' => Ok(Self::VerticalLineBy),
            'C' => Ok(Self::CubicBezierTo),
            'c' => Ok(Self::CubicBezierBy),
            'S' => Ok(Self::SmoothBezierTo),
            's' => Ok(Self::SmoothBezierBy),
            'Q' => Ok(Self::QuadraticBezierTo),
            'q' => Ok(Self::QuadraticBezierBy),
            'T' => Ok(Self::SmoothQuadraticBezierTo),
            't' => Ok(Self::SmoothQuadraticBezierBy),
            'A' => Ok(Self::ArcTo),
            'a' => Ok(Self::ArcBy),
            'Z' | 'z' => Ok(Self::ClosePath),
            _ => Err(oxvg_parse::error::Error::Path(PathError::NoCommand)),
        }
    }
}

fn parse_number<'input>(
    input: &mut oxvg_parse::Parser<'input>,
) -> Result<f64, oxvg_parse::error::Error<'input>> {
    let f = f64::parse(input)?;
    input.skip_whitespace();
    input.skip_char(',');
    input.skip_whitespace();
    Ok(f)
}
fn parse_flag<'input>(
    input: &mut oxvg_parse::Parser<'input>,
) -> Result<f64, oxvg_parse::error::Error<'input>> {
    let f = input.read()?;
    input.skip_whitespace();
    input.skip_char(',');
    input.skip_whitespace();
    match f {
        '0' => Ok(0.0),
        '1' => Ok(1.0),
        _ => Err(oxvg_parse::error::Error::Path(PathError::InvalidArcFlag)),
    }
}
impl command::Data {
    #[allow(clippy::many_single_char_names)]
    fn parse<'input>(
        input: &mut oxvg_parse::Parser<'input>,
        command_id: command::ID,
    ) -> Result<Self, oxvg_parse::error::Error<'input>> {
        match command_id {
            command::ID::ClosePath => return Ok(Self::ClosePath),
            command::ID::Implicit(id) => {
                debug_assert!(!id.is_implicit());
                let result = Self::parse(input, *id)?;
                debug_assert!(!result.is_implicit());
                return Ok(Self::Implicit(Box::new(result)));
            }
            command::ID::None => return Err(oxvg_parse::error::Error::Path(PathError::NoCommand)),
            _ => {}
        }
        let is_arc = matches!(command_id, command::ID::ArcTo | command::ID::ArcBy);
        let a = parse_number(input)?;
        match command_id {
            command::ID::HorizontalLineTo => return Ok(Self::HorizontalLineTo([a])),
            command::ID::HorizontalLineBy => return Ok(Self::HorizontalLineBy([a])),
            command::ID::VerticalLineTo => return Ok(Self::VerticalLineTo([a])),
            command::ID::VerticalLineBy => return Ok(Self::VerticalLineBy([a])),
            _ => {}
        }
        let b = parse_number(input)?;
        match command_id {
            command::ID::LineTo => return Ok(Self::LineTo([a, b])),
            command::ID::LineBy => return Ok(Self::LineBy([a, b])),
            command::ID::MoveTo => return Ok(Self::MoveTo([a, b])),
            command::ID::MoveBy => return Ok(Self::MoveBy([a, b])),
            command::ID::SmoothQuadraticBezierTo => {
                return Ok(Self::SmoothQuadraticBezierTo([a, b]))
            }
            command::ID::SmoothQuadraticBezierBy => {
                return Ok(Self::SmoothQuadraticBezierBy([a, b]))
            }
            _ => {}
        }
        let c = parse_number(input)?;
        let d = if is_arc {
            parse_flag(input)?
        } else {
            parse_number(input)?
        };
        match command_id {
            command::ID::SmoothBezierTo => return Ok(Self::SmoothBezierTo([a, b, c, d])),
            command::ID::SmoothBezierBy => return Ok(Self::SmoothBezierBy([a, b, c, d])),
            command::ID::QuadraticBezierTo => return Ok(Self::QuadraticBezierTo([a, b, c, d])),
            command::ID::QuadraticBezierBy => return Ok(Self::QuadraticBezierBy([a, b, c, d])),
            _ => {}
        }
        let e = if is_arc {
            parse_flag(input)?
        } else {
            parse_number(input)?
        };
        let f = parse_number(input)?;
        match command_id {
            command::ID::CubicBezierTo => return Ok(Self::CubicBezierTo([a, b, c, d, e, f])),
            command::ID::CubicBezierBy => return Ok(Self::CubicBezierBy([a, b, c, d, e, f])),
            _ => {}
        }
        let g = parse_number(input)?;
        match command_id {
            command::ID::ArcTo => Ok(Self::ArcTo([a, b, c, d, e, f, g])),
            command::ID::ArcBy => Ok(Self::ArcBy([a, b, c, d, e, f, g])),
            _ => unreachable!(),
        }
    }
}

#[test]
#[cfg(feature = "default")]
fn test_path_parse() {
    use crate::Path;
    use oxvg_parse::Parse as _;
    // Should parse single command
    insta::assert_snapshot!(Path::parse_string("M 10,50").unwrap());

    // Should parse multiple commands
    insta::assert_snapshot!(
        Path::parse_string("M 10,50 C 20,30 40,50 60,70 C 10,20 30,40 50,60").unwrap()
    );

    // Should parse arc
    insta::assert_snapshot!(Path::parse_string("m-0,1a 25,25 -30 0,1 0,0").unwrap());

    // Should parse implicit
    insta::assert_snapshot!(Path::parse_string(
        "M 10,50 C 1,2 3,4 5,6.5 .1 .2 .3 .4 .5 -.05176e-005"
    )
    .unwrap());

    // Should parse minified
    insta::assert_snapshot!(Path::parse_string("M10 50C1 2 3 4 5 6.5.1.2.3.4.5-5.176e-7").unwrap());

    // Should error when command isn't given
    assert!(Path::parse_string("0,0").is_err());

    // Should error when args are missing
    assert!(Path::parse_string("m1").is_err());

    // Parse arc with decimals as separators
    insta::assert_snapshot!(Path::parse_string("m-0,1a20.8 20.8 0 0 0 5.2.6").unwrap());

    // Parse implicit arc
    insta::assert_snapshot!(Path::parse_string(
        "m-0,1a29.6 29.6 0 01-2 1.5 151.6 151.6 0 01-2.6 1.8"
    )
    .unwrap());
}
