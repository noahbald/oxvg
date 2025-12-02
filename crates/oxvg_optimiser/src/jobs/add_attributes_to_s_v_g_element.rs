use std::collections::BTreeMap;

use oxvg_ast::{
    element::Element,
    is_element,
    visitor::{Context, Visitor},
};
use oxvg_collections::attribute::Attr;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// Adds attributes to SVG elements in the document. This is not an optimisation
/// and will increase the size of SVG documents.
///
/// # Differences to SVGO
///
/// It's not possible to set a *none* value to an attribute. Elements like
/// `<svg data-icon />` are valid in HTML but not XML, so it's only possible to create
/// an attribute like `<svg data-icon="" />`.
///
/// It's also not possible to create React-like syntax. In SVGO it's possible to define
/// an attribute as `{ "key={value}": undefined }` to produce an attribute like
/// `<svg key={value} />`, however in OXVG you have to provide a string value, so it's
/// output would look like `<svg key={value}="" />`.
///
/// # Examples
///
/// Add an attribute with a prefix
///
/// ```
/// use std::collections::BTreeMap;
/// use oxvg_optimiser::{Jobs, AddAttributesToSVGElement};
///
/// let jobs = Jobs {
///   add_attributes_to_s_v_g_element: Some(AddAttributesToSVGElement {
///     attributes: BTreeMap::from([(String::from("prefix:local"), String::from("value"))]),
///   }),
///   ..Jobs::none()
/// };
/// ```
///
/// # Correctness
///
/// This job may visually change documents if the attribute is a presentation attribute
/// or selected via CSS.
///
/// No validation is applied to provided attribute and may produce incorrect or invalid documents.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct AddAttributesToSVGElement {
    /// Pairs of qualified names and attribute values that are assigned to the `svg`
    #[cfg_attr(feature = "wasm", tsify(type = "Record<string, string>"))]
    pub attributes: BTreeMap<String, String>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for AddAttributesToSVGElement {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !element.is_root() || !is_element!(element, Svg) {
            return Ok(());
        }

        for (name, value) in &self.attributes {
            let name = element.parse_attr_id(name);
            if element.has_attribute(&name) {
                continue;
            }
            let value = context.info.allocator.alloc_str(value);
            let attr = Attr::new(name, value);

            element.set_attribute(attr);
        }
        Ok(())
    }
}

#[test]
fn add_attributes_to_s_v_g_element() -> anyhow::Result<()> {
    use crate::test_config;

    // WARN: This output is different to SVGO, and may break SVGs for use in React
    // SVGO: `<svg data-icon className={classes} />`
    // OXVG: `<svg data-icon="" className={classes}="" />`
    //
    // TODO: Maybe we can add a post-processor to remove trailing `=""`
    insta::assert_snapshot!(test_config!(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "", "className={classes}": "" }
        } }"#,
        comment: "Add multiple attributes without value"
    )?);

    // WARN: This output is different to SVGO
    // SVGO: `<svg data-icon />`
    // OXVG: `<svg data-icon="" />`
    insta::assert_snapshot!(test_config!(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "" }
        } }"#,
        comment: "Add single attribute without value"
    )?);

    insta::assert_snapshot!(test_config!(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "focusable": "false", "data-image": "icon" }
        } }"#,
        comment: "Add multiple attributes with values"
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "addAttributesToSvgElement": {
            "attributes": { "data-icon": "" }
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Ignore nested <svg> elements -->
    test
    <svg />
</svg>"#
        ),
    )?);

    Ok(())
}
