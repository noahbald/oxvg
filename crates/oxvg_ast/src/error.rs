//! Error types.

use std::fmt::Display;

#[cfg(feature = "serialize")]
use oxvg_serialize::error::PrinterError;

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
    #[cfg(feature = "serialize")]
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
            #[cfg(feature = "serialize")]
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
