use oxvg_ast::{
    element::Element,
    is_attribute, is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    attribute::{AttributeGroup, AttributeInfo},
    content_type::ContentTypeId,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Remove attributes on groups that won't be inherited by it's children.
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
pub struct RemoveNonInheritableGroupAttrs(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveNonInheritableGroupAttrs {
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
        if !is_element!(element, G) {
            return Ok(());
        }

        element.attributes().retain(|attr| {
            if is_attribute!(attr, VectorEffect) {
                return false;
            }
            let name = attr.name();
            !name
                .attribute_group()
                .contains(AttributeGroup::Presentation)
                || name
                    .info()
                    .contains(AttributeInfo::PresentationNonInheritableGroupAttrs)
                || matches!(name.r#type(), ContentTypeId::Inheritable(_))
        });

        Ok(())
    }
}

impl Default for RemoveNonInheritableGroupAttrs {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_non_inheritable_group_attrs() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeNonInheritableGroupAttrs": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- retain inheritable attrs -->
    <g class="test" clip-path="url(#clip1)" transform="rotate(45)" display="none" opacity="0.5" visibility="visible">
        <path d="M0 0 L 10 20"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeNonInheritableGroupAttrs": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- removes non-inheritable attrs -->
    <g vector-effect="non-scaling-stroke" stroke="blue">
        <path d="M0 0 L 10 20"/>
    </g>
</svg>"#
        ),
    )?);

    Ok(())
}
