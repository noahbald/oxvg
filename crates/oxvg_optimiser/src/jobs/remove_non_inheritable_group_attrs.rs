use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    style::PresentationAttrId,
    visitor::{ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::{AttrsGroups, Group, PRESENTATION_NON_INHERITABLE_GROUP_ATTRS};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveNonInheritableGroupAttrs(bool);

impl<E: Element> Visitor<E> for RemoveNonInheritableGroupAttrs {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(
        &mut self,
        element: &mut E,
        _context: &oxvg_ast::visitor::Context<E>,
    ) -> Result<(), Self::Error> {
        let name = element.qual_name();
        if name.prefix().is_some() || name.local_name().as_ref() != "g" {
            return Ok(());
        }

        element.attributes().retain(|attr| {
            if attr.prefix().is_some() {
                return false;
            }

            let name = attr.local_name();
            PRESENTATION_NON_INHERITABLE_GROUP_ATTRS.contains(name.as_ref())
                || !AttrsGroups::Presentation.set().contains(name.as_ref())
                || PresentationAttrId::from(name.as_ref()).inheritable()
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
