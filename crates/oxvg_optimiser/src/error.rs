use std::{fmt::Display, sync::mpsc::RecvTimeoutError};

use oxvg_ast::{atom::Atom, error::ComputedStylesError};

#[derive(Debug, Clone)]
pub enum PrecheckError<'input> {
    ScriptingNotSupported,
    AnimationNotSupported,
    ConditionalProcessingNotSupported,
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
#[cfg(feature = "napi")]
pub enum PrefixGeneratorError {
    ThreadTimeoutError(RecvTimeoutError),
    ThreadUnknownError,
    NapiError(napi::Status),
    ClientError(String),
}

#[derive(Debug, Clone)]
pub enum JobsError<'input> {
    ComputedStylesError(ComputedStylesError<'input>),
    CleanupValuesPrecision(u8),
    Precheck(PrecheckError<'input>),
    #[cfg(feature = "napi")]
    PrefixGenerator(PrefixGeneratorError),
    InvalidUserSelector(String),
    InvalidUserRegex(regex::Error),
    #[deprecated]
    Generic(String),
}
impl Display for JobsError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ComputedStylesError(e) => e.fmt(f),
            Self::CleanupValuesPrecision(n) => f.write_fmt(format_args!(
                "The float-precision `{n}` is larger than the maximum of 5"
            )),
            Self::Precheck(e) => e.fmt(f),
            #[cfg(feature = "napi")]
            Self::PrefixGenerator(_) => f.write_str("Prefix generate failed to close threads"),
            Self::InvalidUserSelector(e) => {
                f.write_fmt(format_args!("Invalid selector in configuration: {e}"))
            }
            Self::InvalidUserRegex(e) => e.fmt(f),
            Self::Generic(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for JobsError<'_> {}
