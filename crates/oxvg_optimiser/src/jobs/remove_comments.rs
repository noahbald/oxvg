use std::sync::LazyLock;

use oxvg_ast::{node::Node, visitor::Visitor};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde")]
use serde_with::skip_serializing_none;

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::{error::JobsError, utils::regex_memo};

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", skip_serializing_none)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
    #[cfg_attr(feature = "wasm", tsify(type = "{ regex: string }", optional))]
    pub preserve_patterns: Option<Vec<PreservePattern>>,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone)]
pub struct PreservePattern {
    pub regex: String,
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveComments {
    type Error = JobsError<'input>;

    fn comment(&self, comment: &Node<'input, 'arena>) -> Result<(), Self::Error> {
        self.remove_comment(comment)
    }
}

impl RemoveComments {
    fn remove_comment<'input>(&self, comment: &Node<'input, '_>) -> Result<(), JobsError<'input>> {
        let value = comment
            .node_value()
            .expect("Comment nodes should always have a value");

        for pattern in self
            .preserve_patterns
            .as_ref()
            .unwrap_or(&DEFAULT_PRESERVE_PATTERNS)
        {
            if regex_memo::get(&pattern.regex)
                .map_err(JobsError::InvalidUserRegex)?
                .value()
                .is_match(value.as_ref())
            {
                return Ok(());
            }
        }

        comment.remove();
        Ok(())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for PreservePattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let regex = String::deserialize(deserializer)?;
        Ok(Self { regex })
    }
}

#[cfg(feature = "serde")]
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
