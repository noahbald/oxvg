use std::sync::{LazyLock, OnceLock};

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

#[cfg_attr(feature = "napi", napi)]
#[derive(Debug, Clone)]
pub struct PreservePattern {
    pub regex: String,
    parsed_regex_memo: OnceLock<Result<regex::Regex, regex::Error>>,
}

#[cfg(feature = "napi")]
#[napi]
impl PreservePattern {
    #[napi(constructor)]
    pub fn new(regex: String) -> Self {
        Self {
            regex,
            parsed_regex_memo: OnceLock::new(),
        }
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::FromNapiValue for PreservePattern {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let obj = napi::JsObject::from_napi_value(env, napi_val)?;
        let regex = obj.get("regex")?.ok_or_else(|| {
            napi::Error::new(
                napi::Status::InvalidArg,
                "Missing field `PreservePattern.regex`",
            )
        })?;
        Ok(Self {
            regex,
            parsed_regex_memo: OnceLock::new(),
        })
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveComments {
    type Error = String;

    fn comment(&self, comment: &mut <E as Node<'arena>>::Child) -> Result<(), Self::Error> {
        self.remove_comment(comment)
    }
}

impl RemoveComments {
    fn remove_comment<'arena, N: Node<'arena>>(&self, comment: &N) -> Result<(), String> {
        let value = comment
            .node_value()
            .expect("Comment nodes should always have a value");

        for pattern in self
            .preserve_patterns
            .as_ref()
            .unwrap_or(&DEFAULT_PRESERVE_PATTERNS)
        {
            if pattern
                .parsed_regex_memo
                .get_or_init(|| regex::Regex::new(&pattern.regex))
                .as_ref()
                .map_err(ToString::to_string)?
                .is_match(value.as_ref())
            {
                return Ok(());
            }
        }

        comment.remove();
        Ok(())
    }
}

impl<'de> Deserialize<'de> for PreservePattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let regex = String::deserialize(deserializer)?;
        Ok(Self {
            regex,
            parsed_regex_memo: OnceLock::new(),
        })
    }
}

impl Serialize for PreservePattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.regex.serialize(serializer)
    }
}

static DEFAULT_PRESERVE_PATTERNS: LazyLock<Vec<PreservePattern>> = LazyLock::new(|| {
    vec![PreservePattern {
        regex: "^!".to_string(),
        parsed_regex_memo: OnceLock::new(),
    }]
});

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
