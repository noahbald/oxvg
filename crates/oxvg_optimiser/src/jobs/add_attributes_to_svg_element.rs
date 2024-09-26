use std::{collections::BTreeSet, rc::Rc};

use markup5ever::local_name;
use oxvg_ast::Attributes;
use serde::Deserialize;

use crate::{Context, Job};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddAttributesToSVGElement {
    pub attributes: Attributes,
}

impl Job for AddAttributesToSVGElement {
    fn run(&self, node: &Rc<rcdom::Node>, _context: &Context) {
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
    use crate::{test_config, test_config_default_svg_comment};

    // WARN: This output is different to SVGO, and may break SVGs for use in React
    // SVGO: `<svg data-icon className={classes} />`
    // OXVG: `<svg data-icon="" className={classes}="" />`
    //
    // TODO: Maybe we can add a post-processor to remove trailing `=""`
    insta::assert_snapshot!(test_config_default_svg_comment(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "", "className={classes}": "" }
        } }"#,
        "Add multiple attributes without value"
    )?);

    // WARN: This output is different to SVGO
    // SVGO: `<svg data-icon />`
    // OXVG: `<svg data-icon="" />`
    insta::assert_snapshot!(test_config_default_svg_comment(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "" }
        } }"#,
        "Add single attribute without value"
    )?);

    insta::assert_snapshot!(test_config_default_svg_comment(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "focusable": "false", "data-image": "icon" }
        } }"#,
        "Add multiple attributes with values"
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "" }
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Ignore nested <svg> elements
    test
    <svg />
</svg>"#
        ),
    )?);

    Ok(())
}
