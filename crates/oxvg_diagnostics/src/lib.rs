use std::str::Utf8Error;

use miette::{Diagnostic, NamedSource, Report, Result, SourceSpan};
use quick_xml::{escape::EscapeError, events::attributes::AttrError};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("Error parsing SVG!")]
#[diagnostic()]
pub struct SVGErrors {
    #[source_code]
    src: NamedSource<String>,
    #[related]
    errors: Vec<SVGError>,
}

#[derive(Debug)]
pub enum ElementParseError {
    Attribute(AttributeParseError),
    Name(Utf8Error),
}

#[derive(Debug)]
pub enum AttributeParseError {
    AttrError(AttrError),
    Utf8Error(Utf8Error),
}

impl SVGErrors {
    /// Creates a new `SVGErrors` object with the given source code and errors
    pub fn from_errors(src: NamedSource<String>, errors: Vec<SVGError>) -> Self {
        Self { src, errors }
    }

    /// Returns a miette `Result` with an error, if any errors are present
    ///
    /// # Errors
    /// If there are any contained errors
    pub fn emit(self) -> Result<()> {
        if self.errors.is_empty() {
            return Ok(());
        }
        if self.errors.len() == 1 {
            match self.errors.first() {
                Some(e) => e.clone().emit(self.src),
                None => Ok(()),
            }
        } else {
            Err(self.into())
        }
    }
}

#[derive(Debug, PartialEq, Clone, Diagnostic, Error)]
#[error("{label}")]
#[diagnostic()]
pub struct SVGError {
    label: String,
    #[label]
    span: Option<SourceSpan>,
    #[help]
    advice: Option<String>,
    #[label("Caused by this")]
    cause: Option<SourceSpan>,
}

impl SVGError {
    /// Creates a new `SVGError` with an associated label and span
    pub fn new(label: &str, span: Option<SourceSpan>) -> Self {
        SVGError {
            label: label.into(),
            span,
            advice: None,
            cause: None,
        }
    }

    /// Creates a new `SVGError` from the existing, with help text
    pub fn with_advice(self, advice: &str) -> Self {
        Self {
            advice: Some(advice.into()),
            ..self
        }
    }

    /// Creates a new `SVGError` from the existing, with a related cause
    pub fn with_cause(self, cause: SourceSpan) -> Self {
        Self {
            cause: Some(cause),
            ..self
        }
    }

    /// Returns a miette `Result` with an error
    ///
    /// # Errors
    /// always returns error
    pub fn emit(self, src: NamedSource<String>) -> Result<()> {
        let report: Report = self.into();
        Err(report.with_source_code(src))
    }
}

impl From<(quick_xml::Error, usize)> for SVGError {
    /// Convert from a pair of quick-xml error and the position it occured
    fn from(value: (quick_xml::Error, usize)) -> Self {
        use quick_xml::Error::{
            EmptyDocType, EndEventMismatch, EscapeError, InvalidAttr, InvalidPrefixBind, Io,
            NonDecodable, TextNotFound, UnexpectedBang, UnexpectedEof, UnexpectedToken,
            UnknownPrefix, XmlDeclWithoutVersion,
        };

        let (error, position) = value;
        dbg!(&error, &position);
        let position: SourceSpan = position.into();
        match error {
            EmptyDocType => SVGError::new("The doctype has no content", Some(position)),
            EndEventMismatch { expected, found } => SVGError::new(
                &format!("Expected to find closing tag for {expected}, but found {found} instead"),
                Some(position),
            ),
            EscapeError(error) => ( error, value.1 ).into(),
            InvalidAttr(error) => ( error, value.1 ).into(),
            InvalidPrefixBind { prefix, namespace } => {
                let prefix = String::from_utf8_lossy(&prefix);
                let namespace = String::from_utf8_lossy(&namespace);
                SVGError::new(&format!(r#"Cannot bind "{namespace}" to "{prefix}""#), Some(position))
            },
            Io(error) => SVGError::new(&error.kind().to_string(), None),
            NonDecodable(error) => match error {
                Some(error) => SVGError::new(&error.to_string(), Some(position)),
                None => SVGError::new("Couldn't decode file format for some reason.", None),
            },
            TextNotFound => SVGError::new(
                "Expected text here but found something else",
                Some(position),
            ),
            UnexpectedBang(_) => SVGError::new(r#"Unexpected "!" here"#, Some(position)),
            UnexpectedEof(error) | UnexpectedToken(error) => SVGError::new(&error, Some(position)),
            UnknownPrefix(error) => SVGError::new(&format!(r#"The namespace prefix "{}" is unknown"#, String::from_utf8_lossy(&error)), Some(position)),
            XmlDeclWithoutVersion(error) => {
                match error {
                    Some(attribute) => SVGError::new(&format!("Expected xml declaration to start with a `version` attribute, but found {attribute} instead"), Some(position)),
                    None => SVGError::new("Expected xml declaration to start with a `version attribute`", Some(position))
                }
            }
        }
    }
}

impl From<(AttrError, usize)> for SVGError {
    fn from(value: (AttrError, usize)) -> Self {
        use AttrError::{Duplicated, ExpectedEq, ExpectedQuote, ExpectedValue, UnquotedValue};

        let (error, error_position) = value;
        match error {
            Duplicated(position, other_position) => SVGError::new(
                "Found duplicate attributes",
                Some((error_position..position).into()),
            )
            .with_cause(other_position.into()),
            ExpectedEq(position) => {
                dbg!(&error, &error_position);
                SVGError::new("Expected an `=` for this attribute", Some(position.into()))
            }
            ExpectedQuote(position, char) => {
                let char = &[char];
                let char = String::from_utf8_lossy(char);
                SVGError::new(
                    &format!(r#"Expected a quote (`'` or `"`), but found {char} instead"#),
                    Some(position.into()),
                )
            }
            ExpectedValue(position) => {
                if error_position == position {
                    SVGError::new("Expected a value after `=`", Some(position.into()))
                } else {
                    SVGError::new(
                        "Expected a value directly after `=`, but found something else instead",
                        Some((error_position..position).into()),
                    )
                }
            }
            UnquotedValue(position) => SVGError::new(
                "Expected quotes around value",
                Some((error_position..position).into()),
            ),
        }
    }
}

impl From<(EscapeError, usize)> for SVGError {
    fn from(value: (EscapeError, usize)) -> Self {
        use EscapeError::{
            EntityWithNull, InvalidCodepoint, InvalidDecimal, InvalidHexadecimal, TooLongDecimal,
            TooLongHexadecimal, UnrecognizedSymbol, UnterminatedEntity,
        };

        let (error, position) = value;
        match error {
            EntityWithNull(range) => {
                SVGError::new("Entity is a null character", Some(range.into()))
            }
            InvalidCodepoint(point) => SVGError::new(
                &format!("{point} is not a valid unicode codepoint"),
                Some(position.into()),
            ),
            InvalidDecimal(char) => SVGError::new(
                &format!("Found invalid decimal character `{char}`"),
                Some(position.into()),
            ),
            InvalidHexadecimal(char) => SVGError::new(
                &format!("Found invalid hex character `{char}`"),
                Some(position.into()),
            ),
            TooLongDecimal => SVGError::new("Decimal entity is too long", Some(position.into())),
            TooLongHexadecimal => SVGError::new("Hex entity is too long", Some(position.into())),
            UnrecognizedSymbol(range, symbol) => SVGError::new(
                &format!("{symbol} is not a recognised symbol"),
                Some(range.into()),
            ),
            UnterminatedEntity(range) => {
                SVGError::new("Cannot find `;` after `&`", Some(range.into()))
            }
        }
    }
}

impl From<(ElementParseError, usize)> for SVGError {
    fn from(value: (ElementParseError, usize)) -> Self {
        let (error, position) = value;
        match error {
            ElementParseError::Name(error)
            | ElementParseError::Attribute(AttributeParseError::Utf8Error(error)) => {
                SVGError::new(&error.to_string(), Some(position.into()))
            }
            ElementParseError::Attribute(AttributeParseError::AttrError(error)) => {
                (error, position).into()
            }
        }
    }
}
