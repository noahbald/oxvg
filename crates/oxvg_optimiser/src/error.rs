//! Error types.
use std::fmt::Display;

use oxvg_ast::error::ComputedStylesError;
use oxvg_collections::atom::Atom;

#[derive(Debug, Clone)]
/// Represents conditions in which the precheck job dissallows processing of a document
pub enum PrecheckError<'input> {
    /// Document cannot be process due to risk of breaking scripting
    ScriptingNotSupported,
    /// Document cannot be process due to risk of breaking animation
    AnimationNotSupported,
    /// Document cannot be process due to risk of breaking conditional-processing attributes
    ConditionalProcessingNotSupported,
    /// Document cannot be process due to risk of breaking `xlink:href` attributes
    ReferencesExternalXLink(Atom<'input>),
}
impl Display for PrecheckError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScriptingNotSupported => f.write_str("scripting is not supported"),
            Self::AnimationNotSupported => f.write_str("animation is not supported"),
            Self::ConditionalProcessingNotSupported => {
                f.write_str("conditional processing attributes is not supported")
            }
            Self::ReferencesExternalXLink(xlink) => f.write_fmt(format_args!("the `xlink:href` attribute is referencing an external object '{xlink}' which is not supported"))
        }
    }
}
impl std::error::Error for PrecheckError<'_> {}

#[derive(Debug, Clone)]
/// Errors which may be generated when running optimisation jobs
pub enum JobsError<'input> {
    /// There was an issue while trying to query computed styles
    ComputedStylesError(ComputedStylesError<'input>),
    /// There was an issue with the cleanup-values configuration
    CleanupValuesPrecision(u8),
    /// There was an issue while asserting the safety of optimising the document
    Precheck(PrecheckError<'input>),
    /// There was an issue with a selector in the document or configuration
    InvalidUserSelector(String),
    /// There was an issue with a regex string in the configuration
    InvalidUserRegex(regex::Error),
}
impl Display for JobsError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ComputedStylesError(e) => e.fmt(f),
            Self::CleanupValuesPrecision(n) => f.write_fmt(format_args!(
                "The float-precision `{n}` is larger than the maximum of 5"
            )),
            Self::Precheck(e) => e.fmt(f),
            Self::InvalidUserSelector(e) => {
                f.write_fmt(format_args!("Invalid selector in configuration: {e}"))
            }
            Self::InvalidUserRegex(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for JobsError<'_> {}
