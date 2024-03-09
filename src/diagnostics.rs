use miette::{Diagnostic, NamedSource, Report, Result, SourceSpan};
use thiserror::Error;

use crate::{
    cursor::{Cursor, Span},
    markup::TagType,
};

#[derive(Debug, Error, Diagnostic)]
#[error("Error parsing SVG!")]
#[diagnostic()]
pub struct SVGErrors {
    #[source_code]
    src: NamedSource<String>,
    #[related]
    errors: Vec<SVGError>,
}

impl SVGErrors {
    pub fn from_error(src: NamedSource<String>, error: SVGError) -> Self {
        SVGErrors {
            src,
            errors: vec![error],
        }
    }

    pub fn from_errors(src: NamedSource<String>, errors: Vec<SVGError>) -> Self {
        Self { src, errors }
    }

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
    span: SourceSpan,
    #[help]
    advice: Option<String>,
}

impl SVGError {
    pub fn new(label: String, span: SourceSpan) -> Self {
        SVGError {
            label,
            span,
            advice: None,
        }
    }

    pub fn with_advice(self, advice: &str) -> Self {
        Self {
            advice: Some(advice.into()),
            ..self
        }
    }

    pub fn emit(self, src: NamedSource<String>) -> Result<()> {
        let report: Report = self.into();
        Err(report.with_source_code(src))
    }

    pub fn new_curse(_cursor: Cursor, _label: SVGErrorLabel) -> Self {
        todo!("Delete me")
    }

    pub fn new_span(_span: Span, _label: SVGErrorLabel) -> Self {
        todo!("Delete me")
    }
}

#[derive(Debug, Error, Diagnostic, PartialEq, Clone)]
pub enum SVGErrorLabel {
    #[error("Found non-whitespace before first tag")]
    TextBeforeFirstTag,
    #[error("Found text data outside of root tag")]
    TextOutsideRoot,
    #[error("Unencoded `<` found")]
    UnencodedLt,
    #[error("Inappropriately located doctype declaration")]
    InappropriateDoctype,
    #[error("Malformed comment")]
    MalformedComment,
    #[error("Duplicate attribute `{0}`")]
    DuplicateAttribute(String),
    #[error("The file ended before the expected closing svg tag")]
    UnexpectedEndOfFile,
    #[error("Expected the file to end after the closing svg tag")]
    ExpectedEndOfFile,
    #[error("Unexpected character found")]
    #[diagnostic(severity(error), help("Unexpected `{0}` found, expected {1} instead"))]
    UnexpectedChar(char, String),
    #[error("Expected a word here but found a symbol instead")]
    ExpectedWord,
    #[error("Expected whitespace here but found a symbol instead")]
    ExpectedWhitespace,
    #[error("Unexpected {0} tag found")]
    UnexpectedTagType(TagType),
    #[error("The file doesn't contain a root svg element")]
    NoRootElement,
    #[error("The file contains more than 1 root element")]
    MultipleRootElements,
    #[error("Unexpected </{0}>, expected </{1}>")]
    UnmatchedTag(String, String),
    #[error("{0} is not a legal character")]
    IllegalCharRef(String),
    #[error("{0}")]
    Generic(String),
    #[error("Something went wrong with oxvg, please raise a defect: {0}")]
    Internal(String),
}
