use oxvg_ast::{
    attribute::AttributeGroup,
    element::Element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Removes empty attributes from elements when safe to do so.
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
        _document: &Element<'input, 'arena>,
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
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        element.attributes().retain(|a| {
            if a.name()
                .attribute_group()
                .contains(AttributeGroup::ConditionalProcessing)
            {
                return true;
            }
            !a.value().is_empty()
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
    <!-- remove empty attrs -->
    <g attr1="" attr2=""/>
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

    Ok(())
}
