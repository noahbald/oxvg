use lightningcss::{properties::PropertyId, vendor_prefix::VendorPrefix};
use oxvg_ast::{
    attribute::Attributes,
    element::Element,
    get_computed_styles_factory,
    name::Name,
    style::{Id, PresentationAttrId},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::CONTAINER;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Removes container elements with no functional children or meaningful attributes.
///
/// # Correctness
///
/// This job shouldn't visually change the document. Removing whitespace may have
/// an effect on `inline` or `inline-block` elements.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveEmptyContainers(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveEmptyContainers {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::use_style
        } else {
            PrepareOutcome::skip
        })
    }

    fn use_style(&self, element: &E) -> bool {
        element.prefix().is_none() && element.local_name().as_ref() == "g"
    }

    fn exit_element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let name = &element.qual_name().formatter().to_string();
        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);

        if name == "svg" || !CONTAINER.contains(name) || !element.is_empty() {
            return Ok(());
        } else if name == "pattern" {
            if !element.attributes().is_empty() {
                return Ok(());
            }
        } else if name == "mask" {
            if element.has_attribute_local(&"id".into()) {
                return Ok(());
            }
        } else if Element::parent_element(element)
            .is_some_and(|e| e.prefix().is_none() && e.local_name().as_ref() == "switch")
        {
            return Ok(());
        }
        if name == "g" && (get_computed_styles!(Filter(VendorPrefix::None)).is_some()) {
            return Ok(());
        }

        element.remove();
        Ok(())
    }
}

impl Default for RemoveEmptyContainers {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn remove_empty_containers() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyContainers": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove empty containers -->
    <pattern/>
    <g>
        <marker>
            <a/>
        </marker>
    </g>
    <path d="..."/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyContainers": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- preserve non-empty containers -->
    <defs>
        <pattern id="a">
            <rect/>
        </pattern>
        <pattern xlink:href="url(#a)" id="b"/>
    </defs>
    <g>
        <marker>
            <a/>
        </marker>
        <path d="..."/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyContainers": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:x="http://www.w3.org/1999/xlink">
    <!-- preserve non-empty containers -->
    <defs>
        <pattern id="a">
            <rect/>
        </pattern>
        <pattern x:href="url(#a)" id="b"/>
    </defs>
    <g>
        <marker>
            <a/>
        </marker>
        <path d="..."/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyContainers": true }"#,
        Some(
            r#"<svg>
    <!-- preserve non-empty containers -->
    <defs>
        <filter id="feTileFilter" filterUnits="userSpaceOnUse" primitiveUnits="userSpaceOnUse" x="115" y="40" width="250" height="250">
            <feFlood x="115" y="40" width="54" height="19" flood-color="lime"/>
            <feOffset x="115" y="40" width="50" height="25" dx="6" dy="6" result="offset"/>
            <feTile/>
        </filter>
    </defs>
    <g filter="url(#feTileFilter)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyContainers": true }"#,
        Some(
            r#"<svg width="480" height="360" xmlns="http://www.w3.org/2000/svg">
    <!-- preserve id'd mask -->
    <mask id="testMask" />
    <rect x="100" y="100" width="250" height="150" fill="green" />
    <rect x="100" y="100" width="250" height="150" fill="red" mask="url(#testMask)" />
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyContainers": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 462 352">
    <!-- preserve children of `switch` -->
    <switch>
        <g requiredFeatures="http://www.w3.org/TR/SVG11/feature#Extensibility"/>
        <a transform="translate(0,-5)" href="https://www.diagrams.net/doc/faq/svg-export-text-problems" target="_blank">
            <text text-anchor="middle" font-size="10px" x="50%" y="100%">Viewer does not support full SVG 1.1</text>
        </a>
    </switch>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyContainers": true }"#,
        Some(
            r##"<svg viewBox="0 0 50 50" xmlns="http://www.w3.org/2000/svg">
    <!-- preserve filtered `g`s -->
    <filter id="a" x="0" y="0" width="50" height="50" filterUnits="userSpaceOnUse">
        <feFlood flood-color="#aaa"/>
    </filter>
    <mask id="b" x="0" y="0" width="50" height="50">
        <g style="filter: url(#a)"/>
    </mask>
    <text x="16" y="16" style="mask: url(#b)">•ᴗ•</text>
</svg>"##
        ),
    )?);

    Ok(())
}
