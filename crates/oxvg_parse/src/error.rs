//! Error types that may occur while parsing an SVG value

#[derive(Debug, Clone, PartialEq)]
/// An error that can occur while parsing path data
pub enum PathError {
    /// A command was not given when expected
    NoCommand,
    /// A non move command was provided first
    InvalidFirstCommand,
    /// A flag (`0` or `1`) was expected with one two arc flags
    InvalidArcFlag,
    /// A command argument was invalid
    InvalidNumber(std::num::ParseFloatError),
}

/// Parse errors that can be encountered by parsing
#[derive(Debug, Clone, PartialEq)]
pub enum Error<'input> {
    /// The end of an input was reached before parsing finished
    EndOfInput,
    /// An invalid number was parsed.
    InvalidNumber,
    /// A valid value within an invalid range was parsed.
    InvalidRange,
    /// Parsing is done but there is trailing input.
    ExpectedDone,
    /// A specific string was unmatched
    ExpectedString {
        /// The expected string
        expected: &'static str,
        /// The received string
        received: &'input str,
    },
    /// A matchable pattern was unmatched
    ExpectedMatch {
        /// A description of the expected pattern
        expected: &'static str,
        /// The received string
        received: &'input str,
    },
    /// A specific character was unmatched
    ExpectedChar {
        /// The expected character
        expected: char,
        /// The received character
        received: char,
    },
    /// A specific identifier was unmatched
    ExpectedIdent {
        /// The expected identifier(s)
        expected: &'static str,
        /// The received string
        received: &'input str,
    },
    #[cfg(feature = "lightningcss")]
    /// An invalid lightningcss value was parsed
    Lightningcss(
        cssparser_lightningcss::ParseError<'input, lightningcss::error::ParserError<'input>>,
    ),
    /// An invalid path definition was parsed
    Path(PathError),
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt = match self {
            Self::EndOfInput => "Unexpected end of input while parsing",
            Self::InvalidNumber => "Invalid number",
            Self::InvalidRange => "Value out of range",
            Self::ExpectedDone => "Unexpected trailing content after parsing",
            Self::ExpectedString { expected, received } => {
                return f.write_fmt(format_args!(
                    r#"Expected "{expected}" but received "{received}" instead"#
                ))
            }
            Self::ExpectedMatch { expected, received } => {
                return f.write_fmt(format_args!(
                    "Expected value matching {expected} but received {received} instead"
                ))
            }
            Self::ExpectedChar { expected, received } => {
                return f.write_fmt(format_args!(
                    "Expected '{expected}' but received '{received}' instead"
                ))
            }
            Self::ExpectedIdent { expected, received } => {
                return f.write_fmt(format_args!(
                    "Expected {expected} but received `{received}` instead"
                ))
            }
            Self::Lightningcss(e) => return e.fmt(f),
            Self::Path(e) => return e.fmt(f),
        };
        f.write_str(fmt)
    }
}
impl std::error::Error for Error<'_> {}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt = match self {
            Self::NoCommand => "Expected a path command",
            Self::InvalidFirstCommand => "Expected path to start with `m` or `M`",
            Self::InvalidArcFlag => "Expected binary digit (`0` or `1`) for arc flag",
            Self::InvalidNumber(e) => &format!("Failed to parse number in path: {e}"),
        };
        f.write_str(fmt)?;
        Ok(())
    }
}
impl std::error::Error for PathError {}
