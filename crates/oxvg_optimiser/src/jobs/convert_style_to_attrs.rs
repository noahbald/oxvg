use std::collections::HashMap;

use lightningcss::properties::Property;
use oxvg_ast::{
    attribute::data::{Attr, AttrId},
    element::Element,
    get_attribute_mut,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::{error::JobsError, utils::minify_style};

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

impl<'input, 'arena> Visitor<'input, 'arena> for ConvertStyleToAttrs {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let Some(mut styles_attr) = get_attribute_mut!(element, Style) else {
            return Ok(());
        };
        let styles = &mut styles_attr.0;

        minify_style::style(styles);

        let mut attribute_insertions: HashMap<AttrId<'input>, usize> = HashMap::new();
        let mut new_attributes: Vec<Attr<'input>> = Vec::new();

        let mut detain_and_collect_presentation_attrs = |property: &Property<'input>| {
            let attr = match property.clone().try_into().ok() {
                None | Some(Attr::CSSUnknown { .. } | Attr::Unparsed { .. }) => return true,
                Some(attr) => attr,
            };
            let name = attr.name();
            if attribute_insertions.contains_key(name) {
                let index = attribute_insertions[name];
                new_attributes[index] = attr;
            } else {
                attribute_insertions.insert(name.clone(), new_attributes.len());
                new_attributes.push(attr);
            }
            false
        };

        styles
            .declarations
            .retain(&mut detain_and_collect_presentation_attrs);
        if !self.keep_important {
            styles
                .important_declarations
                .retain(detain_and_collect_presentation_attrs);
        }

        let is_empty = styles.is_empty();
        drop(styles_attr);
        if is_empty {
            element.remove_attribute(&AttrId::Style);
        }

        for value in new_attributes {
            element.set_attribute(value);
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
