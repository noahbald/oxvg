use std::collections::BTreeMap;

use oxvg_ast::element::Element;
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Context, Job, JobDefault};

#[derive(Deserialize, Default, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
#[job_default(is_default = false)]
pub struct AddAttributesToSVGElement {
    pub attributes: BTreeMap<String, String>,
}

impl<E: Element> Job<E> for AddAttributesToSVGElement {
    fn run(&self, element: &E, _context: &Context<E>) {
        let name = element.local_name();

        if !element.is_root() || name.as_ref() != "svg" {
            return;
        };

        for (name, value) in &self.attributes {
            let name = name.as_str().into();
            let value = value.as_str().into();
            if element.has_attribute(&name) {
                continue;
            }

            element.set_attribute(name, value);
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
