//! Error types that may occur while parsing an SVG value

/// A parser error
#[derive(Debug)]
pub enum ParseErrorKind<'i> {
    /// Lightningcss failed to parse
    CSSParserError(lightningcss::error::ParserError<'i>),
    /// A fundamental parsing step failed
    Basic(cssparser_lightningcss::BasicParseError<'i>),
    /// A clock value didn't fit the expected range
    InvalidClockValue,
    /// A unexpected set of paint steps were given
    InvalidPaintOrder,
    /// A begin-end syncbase value is missing an id
    MissingSyncbaseId,
}

/// Parse errors that can be encoutered by parsing
pub type ParseError<'input> = cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>;

impl<'input> ParseErrorKind<'input> {
    /// Maps a lightnincss error to `Self`
    pub fn from_css(
        error: cssparser_lightningcss::ParseError<'input, lightningcss::error::ParserError<'input>>,
    ) -> cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>> {
        match error.kind {
            cssparser_lightningcss::ParseErrorKind::Basic(e) => {
                cssparser_lightningcss::ParseError {
                    kind: cssparser_lightningcss::ParseErrorKind::Basic(e),
                    location: error.location,
                }
            }
            cssparser_lightningcss::ParseErrorKind::Custom(e) => {
                cssparser_lightningcss::ParseError {
                    kind: cssparser_lightningcss::ParseErrorKind::Custom(
                        ParseErrorKind::CSSParserError(e),
                    ),
                    location: error.location,
                }
            }
        }
    }

    /// Maps a basic error to `Self`
    pub fn from_basic(
        error: cssparser_lightningcss::BasicParseError<'input>,
    ) -> cssparser_lightningcss::ParseError<'input, Self> {
        cssparser_lightningcss::ParseError {
            kind: cssparser_lightningcss::ParseErrorKind::Basic(error.kind),
            location: error.location,
        }
    }

    /// Maps a parser error to `Self`
    pub fn from_parser(
        error: cssparser_lightningcss::ParseError<'input, cssparser_lightningcss::BasicParseError>,
    ) -> cssparser_lightningcss::ParseError<'input, Self> {
        match error.kind {
            cssparser_lightningcss::ParseErrorKind::Basic(e) => {
                cssparser_lightningcss::ParseError {
                    kind: cssparser_lightningcss::ParseErrorKind::Basic(e),
                    location: error.location,
                }
            }
            cssparser_lightningcss::ParseErrorKind::Custom(e) => {
                cssparser_lightningcss::ParseError {
                    kind: cssparser_lightningcss::ParseErrorKind::Custom(ParseErrorKind::Basic(e)),
                    location: error.location,
                }
            }
        }
    }
}
