use std::collections::BTreeMap;

use lightningcss::{
    printer::PrinterOptions,
    properties::Property,
    stylesheet::{MinifyOptions, ParserOptions, StyleAttribute},
};
use oxvg_ast::{
    attribute::Attr as _,
    element::Element,
    name::Name,
    style::PresentationAttrId,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
/// Converts presentation attributes in element styles to the equivalent XML attribute.
///
/// Presentation attributes can be used in both attributes and styles, but in most cases it'll take fewer
/// bytes to use attributes. Consider the following:
///
/// ```xml
/// <rect width="100" height="100" style="fill:red"/>
/// <!-- vs -->
/// <rect width="100" height="100" fill="red"/>
/// ```
///
/// However, because the `style` attribute doesn't require quotes between values, given enough
/// presentation attributes, it can increase the size of the document.
///
/// ```xml
/// <rect width="100" height="100" style="fill:red;opacity:.5;stroke-dasharray:1;stroke:blue;stroke-opacity:.5"/>
/// <!-- vs -->
/// <rect width="100" height="100" fill="red" opacity=".5" stroke-dasharray="1" stroke="blue" stroke-opacity=".5"/>
/// ```
///
/// # Differences to SVGO
///
/// Unlike SVGO this job doesn't attempt to cleanup broken style attributes.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct ConvertStyleToAttrs {
    #[serde(default = "default_keep_important")]
    /// Whether to always keep `!important` styles.
    pub keep_important: bool,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for ConvertStyleToAttrs {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        let style_name = &"style".into();
        let Some(mut style_attr) = element.get_attribute_node_local_mut(style_name) else {
            return Ok(());
        };

        let style_attr_value = style_attr.value().clone();
        let Ok(mut styles) =
            StyleAttribute::parse(style_attr_value.as_ref(), ParserOptions::default())
        else {
            return Ok(());
        };
        styles.minify(MinifyOptions::default());

        let mut new_attributes: BTreeMap<<E::Name as Name>::LocalName, E::Atom> = BTreeMap::new();

        let mut detain_and_collect_presentation_attrs = |property: &Property| {
            let property_id = property.property_id();
            let name = property_id.name();
            let presentation_attr_id = PresentationAttrId::from(name);
            if matches!(presentation_attr_id, PresentationAttrId::Unknown(_)) {
                true
            } else if let Ok(value) = property.value_to_css_string(PrinterOptions::default()) {
                new_attributes.insert(name.into(), value.into());
                false
            } else {
                true
            }
        };

        styles
            .declarations
            .declarations
            .retain(&mut detain_and_collect_presentation_attrs);
        if !self.keep_important {
            styles
                .declarations
                .important_declarations
                .retain(detain_and_collect_presentation_attrs);
        }

        let Ok(result) = styles.to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        }) else {
            return Ok(());
        };
        if styles.declarations.declarations.is_empty()
            && styles.declarations.important_declarations.is_empty()
        {
            drop(style_attr);
            element.remove_attribute_local(style_name);
        } else {
            style_attr.set_value(result.code.into());
            drop(style_attr);
        }

        for (local_name, value) in new_attributes {
            element.set_attribute_local(local_name, value);
        }

        Ok(())
    }
}

const fn default_keep_important() -> bool {
    false
}

#[test]
fn convert_style_to_attrs() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertStyleToAttrs": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- move style to attributes -->
    <g style="fill:#000;"/>
    <g style="font-family:'Helvetica Neue'"/>
    <g style="    fill:#000; color: #fff  ;  "/>
</svg>"#
        ),
    )?);

    // NOTE: Different to SVGO
    insta::assert_snapshot!(test_config(
        r#"{ "convertStyleToAttrs": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- cannot change broken attribute -->
    <g style="    fill:#000; c\olor: #fff; /**/illegal-'declaration/*'; -webkit-blah: 123  ; -webkit-trolo: 'lolo'; illegal2*/"/>
    <g style="font:15px serif"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertStyleToAttrs": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- handle inline comments and urls -->
    <g style="background/*-image*/:url(data:image/png;base64,iVBORw...)"/>
    <g style="fill:url(data:image/png;base64,iVBORw...)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertStyleToAttrs": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- can move important styles -->
    <rect width="100" height="100" class="blue red" style="fill:red!important"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertStyleToAttrs": { "keepImportant": true } }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- cannot move important styles -->
    <rect width="100" height="100" class="blue red" style="fill:red!important"/>
</svg>"#
        ),
    )?);

    Ok(())
}
