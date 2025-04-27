use itertools::Itertools;
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Removes redundant whitespace from attribute values.
///
/// # Correctness
///
/// By default any whitespace is cleaned up. This shouldn't affect anything within the SVG
/// but may affect elements within `<foreignObject />`, which is treated as HTML.
///
/// For example, whitespace has an effect when between `inline` and `inline-block` elements.
/// See [MDN](https://developer.mozilla.org/en-US/docs/Web/API/Document_Object_Model/Whitespace#spaces_in_between_inline_and_inline-block_elements) for more information.
///
/// In any other case, it should never affect the appearance of the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct CleanupAttributes {
    #[serde(default = "newlines_default")]
    /// Whether to replace `'\n'` with `' '`.
    pub newlines: bool,
    #[serde(default = "trim_default")]
    /// Whether to remove whitespace from each end of the value
    pub trim: bool,
    #[serde(default = "spaces_default")]
    /// Whether to replace multiple whitespace characters with a single `' '`.
    pub spaces: bool,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for CleanupAttributes {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        for mut attr in element.attributes().into_iter_mut() {
            let mut value = attr.value().to_string();
            let original_len = value.len();
            if self.newlines {
                value = value.replace('\n', " ");
            }
            let value = if self.trim {
                value.trim()
            } else {
                value.as_ref()
            };
            if self.spaces {
                let value = value.split_whitespace().join(" ");
                attr.set_value(value.into());
            } else if value.len() < original_len {
                attr.set_value(value.into());
            }
        }
        Ok(())
    }
}

impl Default for CleanupAttributes {
    fn default() -> Self {
        Self {
            newlines: newlines_default(),
            trim: trim_default(),
            spaces: spaces_default(),
        }
    }
}

const fn newlines_default() -> bool {
    true
}

const fn trim_default() -> bool {
    true
}

const fn spaces_default() -> bool {
    true
}

#[test]
fn cleanup_attributes() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupAttributes": {
            "newlines": true,
            "trim": true,
            "spaces": true
        } }"#,
        Some(
            r#"<svg xmlns="  http://www.w3.org/2000/svg
  " attr="a      b" attr2="a
b">
    <!-- Should remove all unnecessary whitespace from attributes -->
    test
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupAttributes": {
            "newlines": true,
            "trim": true,
            "spaces": true
        } }"#,
        Some(
            r#"<svg xmlns="  http://www.w3.org/2000/svg
  " attr="a      b">
    <!-- Should remove all unnecessary whitespace from attributes -->
    test &amp; &lt;&amp; &gt; &apos; &quot; &amp;
</svg>"#
        )
    )?);

    Ok(())
}
