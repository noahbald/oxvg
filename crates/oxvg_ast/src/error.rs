//! Error types.

use std::fmt::Display;

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

/// A printer error
pub type PrinterError = lightningcss::error::PrinterError;

/// An error while serializing a document.
#[derive(Debug)]
pub enum XmlWriterError {
    /// An error while running an io operation.
    IO(std::io::Error),
    /// An error while formatting
    FMT(std::fmt::Error),
    /// An error while flushing buffer.
    BufWriter(std::io::IntoInnerError<std::io::BufWriter<Vec<u8>>>),
    /// An error after writing to string.
    UTF8(std::string::FromUtf8Error),
    /// An error while serializing an attribute
    PrinterError(PrinterError),
    /// Did not have opening element name when closing element.
    ClosedUnopenedElement,
    /// Attempted to write attribute before `start_element()` or after `close_element()`.
    AttributeWrittenBeforeElement,
    /// Declaration was already written.
    DeclarationAlreadyWritten,
    /// Attempts to write text before `start_element()`.
    TextBeforeElement,
    /// Attempts to write CDATA with `]]>` in the content.
    BadCDATA,
}
impl std::fmt::Display for XmlWriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(err) => err.fmt(f),
            Self::FMT(err) => err.fmt(f),
            Self::BufWriter(err) => err.fmt(f),
            Self::UTF8(err) => err.fmt(f),
            Self::PrinterError(err) => err.fmt(f),
            Self::ClosedUnopenedElement => {
                "Did not have opening element name when closing element.".fmt(f)
            }
            Self::AttributeWrittenBeforeElement => {
                "Attempted to write attribute before `start_element()` or after `close_element()`."
                    .fmt(f)
            }
            Self::TextBeforeElement => "Attempts to write text before `start_element()`.".fmt(f),
            Self::BadCDATA => "Attempts to write CDATA with `]]>` in the content.".fmt(f),
            Self::DeclarationAlreadyWritten => "Declaration was already written.".fmt(f),
        }
    }
}
impl std::error::Error for XmlWriterError {}

/// An error while gathering computed styles
#[derive(Debug, Clone)]
pub enum ComputedStylesError<'input> {
    /// A selector was found that may affect validity of computed styles
    BadSelector(lightningcss::selector::SelectorList<'input>),
}
impl Display for ComputedStylesError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSelector(selector) => {
                f.write_str("Found an invalid selector while processing: ")?;
                selector.fmt(f)
            }
        }
    }
}
impl std::error::Error for ComputedStylesError<'_> {}
