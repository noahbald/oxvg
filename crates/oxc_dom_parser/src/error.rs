//! Error management.

use std::fmt::Display;

use crate::Span;

pub type ParseResult<T> = Result<T, Error>;

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Span,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// An error pointing at a single byte offset.
    pub(crate) fn point(kind: ErrorKind, at: u32) -> Self {
        Self {
            kind,
            span: Span::new(at, at + 1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    SourceError(SourceError),
    UnrecognisedData,
    UnexpectedEndOfFile,
    UnexpectedCloseTag,
    UnexpectedDoctype,
    UnexpectedNonWhitespace,
    DisallowedUnquotedAttributeValue,
    DisallowedMissingAttributeValue,
    DisallowedMultipleRootElements,
    DisallowedDuplicateAttributes,
    InvalidAttributeCharacter,
    InvalidReference,
    InvalidDoctypeData,
    UnknownReference,
    MismatchedCloseTag,
    MissingRootElement,
    DTDDetected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceError {
    // The source exceeds the maximum supported size (4GiB).
    TooLong,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}..{}", self.kind, self.span.start, self.span.end)
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceError(err) => err.fmt(f),
            Self::UnrecognisedData => f.write_str("Invalid chunk of data"),
            Self::UnexpectedEndOfFile => f.write_str("Unexpected end-of-file"),
            Self::UnexpectedCloseTag => f.write_str("Unexpected close of tag"),
            Self::UnexpectedDoctype => f.write_str("Unexpected DOCTYPE"),
            Self::DisallowedUnquotedAttributeValue => {
                f.write_str("Unquoted attribute values not allowed")
            }
            Self::DisallowedMissingAttributeValue => {
                f.write_str("Omitted attribute values not allowed")
            }
            Self::DisallowedMultipleRootElements => {
                f.write_str("Multiple root elements not allowed")
            }
            Self::DisallowedDuplicateAttributes => {
                f.write_str("Multiple attributes of the same name not allowed on elements")
            }
            Self::InvalidAttributeCharacter => f.write_str("Invalid character found in attribute"),
            Self::InvalidReference => f.write_str("Invalid entity reference"),
            Self::InvalidDoctypeData => f.write_str("Invalid doctype data"),
            Self::UnknownReference => f.write_str("Unknown entity reference"),
            Self::MismatchedCloseTag => f.write_str("Close tag doesn't match opening tag name"),
        }
    }
}

impl Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLong => f.write_str("source exceeds the maximum supported size (4 GiB)"),
        }
    }
}

impl std::error::Error for Error {}
