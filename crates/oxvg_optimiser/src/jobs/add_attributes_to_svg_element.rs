use std::{collections::BTreeSet, rc::Rc};

use markup5ever::local_name;
use oxvg_ast::Attributes;
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddAttributesToSVGElement {
    pub attributes: Attributes,
}

impl Job for AddAttributesToSVGElement {
    fn run(&self, node: &Rc<rcdom::Node>) {
        use rcdom::NodeData::Element;

        let Element { attrs, name, .. } = &node.data else {
            return;
        };

        let is_root = oxvg_selectors::Element::new(node.clone()).is_root();
        if name.local != local_name!("svg") || !is_root {
            return;
        };
        let attrs = &mut *attrs.borrow_mut();
        let keys: BTreeSet<_> = attrs.iter().map(|attr| attr.name.clone()).collect();

        for attr in &Into::<Vec<markup5ever::Attribute>>::into(&self.attributes) {
            let key = &attr.name;
            if keys.contains(key) {
                continue;
            }
            attrs.push(attr.clone());
        }
    }
}

#[test]
fn add_attributes_to_svg_element() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        // Add multiple attributes without value
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "", "className={classes}": "" }
        } }"#,
        None,
    )?);

    insta::assert_snapshot!(test_config(
        // Add single attribute without value
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "" }
        } }"#,
        None,
    )?);

    insta::assert_snapshot!(test_config(
        // Add multiple attributes with values
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "focusable": "false", "data-image": "icon" }
        } }"#,
        None,
    )?);

    insta::assert_snapshot!(test_config(
        // Ignore nested <svg> elements
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "" }
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
    <svg />
</svg>"#
        ),
    )?);

    Ok(())
}
