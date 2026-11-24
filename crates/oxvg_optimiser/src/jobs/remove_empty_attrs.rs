use oxvg_ast::{
    element::Element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::attribute::AttributeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(transparent))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(transparent))]
/// Removes empty attributes from elements when safe to do so.
///
/// # Differences to SVGO
///
/// SVGO removes empty attributes that an element doesn't expect, such as
/// `rdf:about` on `cc:Work`, and some whose empty value means something, such
/// as `xml:lang`. We keep both.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveEmptyAttrs(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveEmptyAttrs {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        // Keep an empty attribute whose empty value means something, such as
        // `systemLanguage=""`, or that the element doesn't expect, such as
        // `rdf:about=""` for this document.
        element.attributes().retain(|a| {
            !a.value().is_empty()
                || a.name().info().contains(AttributeInfo::EmptySemantics)
                || !element.qual_name().is_permitted_attribute(a.name())
        });
        Ok(())
    }
}

impl Default for RemoveEmptyAttrs {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_empty_attrs() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyAttrs": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove empty attrs, but not unknown ones -->
    <g attr1="" attr2="" class="" id=""/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyAttrs": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- retain conditional processing attrs -->
    <g requiredFeatures=""/>
    <g requiredExtensions=""/>
    <g systemLanguage=""/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyAttrs": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:cc="http://creativecommons.org/ns#">
    <!-- keep empty attrs that mean something or are unknown -->
    <metadata>
        <rdf:RDF><cc:Work rdf:about=""/></rdf:RDF>
    </metadata>
    <g class="" attr1="" xml:lang="" lang="" />
</svg>"#
        ),
    )?);

    Ok(())
}
