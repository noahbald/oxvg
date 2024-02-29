use miette::{Diagnostic, NamedSource, Result, SourceSpan};
use thiserror::Error;

use crate::{
    cursor::{Cursor, Span},
    markup::TagType,
};

#[derive(Debug, Error, Diagnostic)]
#[error("Error parsing SVG!")]
pub struct SvgParseErrorProvider {
    #[source_code]
    src: NamedSource<String>,
    #[label]
    span: Option<SourceSpan>,
    #[related]
    error: Vec<SvgParseErrorMessage>,
}

impl SvgParseErrorProvider {
    pub fn new_error(
        span: SourceSpan,
        src: NamedSource<String>,
        error: SvgParseErrorMessage,
    ) -> Self {
        SvgParseErrorProvider {
            span: Some(span),
            src,
            error: vec![error],
        }
    }

    pub fn add_error(&mut self, span: SourceSpan, error: SvgParseErrorMessage) {
        if self.span.is_none() {
            self.span = Some(span);
        }
        self.error.push(error);
    }
}

#[derive(Debug, PartialEq)]
pub struct SvgParseError {
    span: Option<Span>,
    cursor: Option<Cursor>,
    message: SvgParseErrorMessage,
}

impl SvgParseError {
    pub fn new_span(span: Span, message: SvgParseErrorMessage) -> Self {
        Self {
            span: Some(span),
            cursor: None,
            message,
        }
    }

    pub fn new_curse(cursor: Cursor, message: SvgParseErrorMessage) -> Self {
        Self {
            span: None,
            cursor: Some(cursor),
            message,
        }
    }

    pub fn source_span(&self, file: &str) -> SourceSpan {
        match &self.span {
            Some(span) => span.as_source_span(file),
            None => self
                .cursor
                .map(|cursor| (cursor.as_source_offset(file), 0).into())
                .unwrap(),
        }
    }

    pub fn as_provider(&self, path: String, file: &str) -> Result<()> {
        let span: SourceSpan = self.source_span(file);
        Err(SvgParseErrorProvider::new_error(
            span,
            NamedSource::new(path, file.to_string()),
            self.message.clone(),
        ))?
    }
}

#[derive(Debug, Error, Diagnostic, PartialEq, Clone)]
pub enum SvgParseErrorMessage {
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
