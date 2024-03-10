use miette::{Diagnostic, NamedSource, Report, Result, SourceSpan};
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

impl SVGErrors {
    /// Creates a new `SVGErrors` object with the given source code and errors
    pub fn from_errors(src: NamedSource<String>, errors: Vec<SVGError>) -> Self {
        Self { src, errors }
    }

    /// Returns a miette `Result` with an error, if any errors are present
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
    /// Creates a new `SVGError` with an associated label and span
    pub fn new(label: String, span: SourceSpan) -> Self {
        SVGError {
            label,
            span,
            advice: None,
        }
    }

    /// Creates a new `SVGError` from the existing, with help text
    pub fn with_advice(self, advice: &str) -> Self {
        Self {
            advice: Some(advice.into()),
            ..self
        }
    }

    /// Returns a miette `Result` with an error
    pub fn emit(self, src: NamedSource<String>) -> Result<()> {
        let report: Report = self.into();
        Err(report.with_source_code(src))
    }
}
