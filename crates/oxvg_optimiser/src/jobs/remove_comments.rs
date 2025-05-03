use oxvg_ast::{element::Element, node::Node, visitor::Visitor};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[cfg_attr(feature = "napi", napi(object))]
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
/// Removes XML comments from the document.
///
/// By default this job ignores comments starting with `<!--!` which is often used
/// for legal information, such as copyright, licensing, or attribution.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// Scripts which target comments, or conditional comments such as `<!--[if IE 8]>`
/// may be affected.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveComments {
    /// A list of regex patters to match against comments, where matching comments will
    /// not be removed from the document.
    pub preserve_patterns: Option<Vec<PreservePattern>>,
}

#[derive(Debug, Clone)]
pub struct PreservePattern(pub regex::Regex);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveComments {
    type Error = String;

    fn comment(&self, comment: &mut <E as Node<'arena>>::Child) -> Result<(), Self::Error> {
        self.remove_comment(comment);
        Ok(())
    }
}

impl RemoveComments {
    fn remove_comment<'arena, N: Node<'arena>>(&self, comment: &N) {
        let value = comment
            .node_value()
            .expect("Comment nodes should always have a value");

        if self
            .preserve_patterns
            .as_ref()
            .unwrap_or(&DEFAULT_PRESERVE_PATTERNS)
            .iter()
            .any(|pattern| pattern.0.is_match(value.as_ref()))
        {
            return;
        }

        comment.remove();
    }
}

#[derive(Debug)]
enum DeserializePreservePatternError {
    InvalidRegex,
}

impl std::fmt::Display for DeserializePreservePatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRegex => f.write_str("expected valid regex string"),
        }
    }
}

impl serde::de::StdError for DeserializePreservePatternError {}

impl<'de> Deserialize<'de> for PreservePattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        let regex = regex::Regex::new(&string)
            .map_err(|_| serde::de::Error::custom(DeserializePreservePatternError::InvalidRegex))?;
        Ok(Self(regex))
    }
}

impl Serialize for PreservePattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.to_string().serialize(serializer)
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::TypeName for PreservePattern {
    fn type_name() -> &'static str {
        "PreservePattern"
    }

    fn value_type() -> napi::ValueType {
        napi::ValueType::String
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::FromNapiValue for PreservePattern {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let string = String::from_napi_value(env, napi_val)?;

        let regex = regex::Regex::new(&string)
            .map_err(|err| napi::Error::new(napi::Status::InvalidArg, err))?;
        Ok(Self(regex))
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ToNapiValue for PreservePattern {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        napi::bindgen_prelude::ToNapiValue::to_napi_value(env, val.0.to_string())
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ValidateNapiValue for PreservePattern {}

lazy_static! {
    static ref DEFAULT_PRESERVE_PATTERNS: Vec<PreservePattern> =
        vec![PreservePattern(regex::Regex::new("^!").unwrap())];
}

#[test]
fn remove_comments() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeComments": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!--- test -->
    <g>
        <!--- test -->
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeComments": {} }"#,
        Some(
            r#"<!--!Icon Font v1 by @iconfont - Copyright 2023 Icon Font CIC.-->
<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeComments": { "preservePatterns": [] } }"#,
        Some(
            r#"<!--!Icon Font v1 by @iconfont - Copyright 2023 Icon Font CIC.-->
<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    Ok(())
}
