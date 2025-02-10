use itertools::Itertools;
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupAttributes {
    newlines: Option<bool>,
    trim: Option<bool>,
    spaces: Option<bool>,
}

impl<E: Element> Visitor<E> for CleanupAttributes {
    type Error = String;

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        for mut attr in element.attributes().into_iter_mut() {
            let mut value = attr.value().to_string();
            let original_len = value.len();
            if matches!(self.newlines, Some(true)) {
                value = value.replace('\n', " ");
            }
            let value = if matches!(self.trim, Some(true)) {
                value.trim()
            } else {
                value.as_ref()
            };
            if matches!(self.spaces, Some(true)) {
                let value = value.split_whitespace().join(" ");
                attr.set_value(value.into());
            } else if value.len() < original_len {
                attr.set_value(value.into());
            }
        }
        Ok(())
    }
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
