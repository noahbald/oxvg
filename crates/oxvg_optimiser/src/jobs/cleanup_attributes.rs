use std::rc::Rc;

use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupAttributes {
    newlines: Option<bool>,
    trim: Option<bool>,
    spaces: Option<bool>,
}

impl Job for CleanupAttributes {
    fn run(&self, node: &Rc<rcdom::Node>) {
        use rcdom::NodeData::Element;

        let Element { attrs, .. } = &node.data else {
            return;
        };
        let attrs = &mut *attrs.borrow_mut();

        for attr in attrs.iter_mut() {
            if matches!(self.newlines, Some(true)) {
                attr.value = attr.value.replace('\n', " ").into();
            }
            if matches!(self.trim, Some(true)) {
                attr.value = attr.value.trim().into();
            }
            if matches!(self.spaces, Some(true)) {
                attr.value = attr
                    .value
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .into();
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
