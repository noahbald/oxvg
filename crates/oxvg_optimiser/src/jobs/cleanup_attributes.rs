use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Context, Job, JobDefault};

#[derive(Deserialize, Default, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct CleanupAttributes {
    newlines: Option<bool>,
    trim: Option<bool>,
    spaces: Option<bool>,
}

impl<E: Element> Job<E> for CleanupAttributes {
    fn run(&self, element: &E, _context: &Context<E>) {
        for mut attr in element.attributes().iter() {
            if matches!(self.newlines, Some(true)) {
                attr.set_value(attr.value().as_ref().replace('\n', " ").into());
            }
            if matches!(self.trim, Some(true)) {
                attr.set_value(attr.value().as_ref().trim().into());
            }
            if matches!(self.spaces, Some(true)) {
                attr.set_value(
                    attr.value()
                        .as_ref()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .into(),
                );
            }
        }
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
