//! Error types.
use oxvg_collections::{atom::Atom, attribute::AttrId, element::ElementId};

#[derive(Debug)]
/// Errors that may occur during processing.
pub enum Error<'input> {
    /// Errors encountered while building JSX
    BuildError(BuildError<'input>),
    /// Errors encountered while parsing config
    ConfigError(ConfigError),
    #[cfg(feature = "optimise")]
    /// Errors encountered while optimising the original document
    OptimiseError(oxvg_optimiser::error::JobsError<'input>),
    /// Errors encountered while executing user template
    TemplateError(TemplateError),
}

#[derive(Debug)]
/// Errors encountered while building JSX
pub enum BuildError<'input> {
    /// Builder entered an unreachable state
    Unreachable,
    /// SVG document contained non-representable xmlns
    UnsupportedXMLNS(String),
    /// SVG document contained non-representable xml prefix
    UnknownXMLPrefixAttr(AttrId<'input>),
    /// SVG document contained non-representable xml prefix
    UnknownXMLPrefixElement(ElementId<'input>),
    /// SVG document contained a non-representable xml name
    InvalidJSXName(Atom<'input>),
    /// Builder produced invalid JSX
    InvalidJSX,
    /// SVG document doesn't contain an SVG element
    MissingSVGElement,
    /// Builder was unable to serialize JSX or SVG data
    PrinterError,
}

#[derive(Debug)]
/// Errors encountered while parsing config
pub enum ConfigError {
    /// Config contained an identifier that is invalid
    InvalidIdent(String),
    /// Config contained an expression that is invalid
    InvalidExpr(String),
}

/// Errors encountered while executing template
#[derive(Debug)]
pub struct TemplateError;

impl std::error::Error for Error<'_> {}
impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuildError(err) => {
                f.write_fmt(format_args!("Error while building document: {err}"))
            }
            Self::ConfigError(err) => {
                f.write_fmt(format_args!("Error while processing config: {err}"))
            }
            #[cfg(feature = "optimise")]
            Self::OptimiseError(err) => {
                f.write_fmt(format_args!("Error while optimising document: {err}"))
            }
            Self::TemplateError(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for BuildError<'_> {}
impl std::fmt::Display for BuildError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unreachable => "Unreachable state occurred while processing JSX",
            Self::UnsupportedXMLNS(uri) => {
                return f.write_fmt(format_args!(
                    "A non-jsx-representable xmlns declaration detected: {uri}"
                ))
            }
            Self::UnknownXMLPrefixAttr(name) => {
                return f.write_fmt(format_args!(
                    "A non-jsx-representable attribute detected: {name}"
                ))
            }
            Self::UnknownXMLPrefixElement(name) => {
                return f.write_fmt(format_args!(
                    "A non-jsx-representable element detected: {name}"
                ))
            }
            Self::InvalidJSXName(name) => {
                return f.write_fmt(format_args!(
                    "The attribute `{name}` cannot be represented in jsx or is not a valid prop of it's element"
                ))
            }
            Self::MissingSVGElement => "Document is missing `<svg>` element",
            Self::InvalidJSX => "Invalid JSX created",
            Self::PrinterError => "Error while serializing into JSX",
        })
    }
}

impl std::error::Error for ConfigError {}
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdent(ident) => {
                f.write_fmt(format_args!("`{ident}` is not a valid identifier"))
            }
            Self::InvalidExpr(expr) => {
                f.write_fmt(format_args!("`{{{expr}}}` is not a valid expression"))
            }
        }
    }
}

impl std::error::Error for TemplateError {}
impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Error while executing template")
    }
}
