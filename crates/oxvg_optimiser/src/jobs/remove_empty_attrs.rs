use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::CONDITIONAL_PROCESSING;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
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

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveEmptyAttrs {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        element.attributes().retain(|a| {
            if a.value().as_ref() != "" {
                return true;
            }

            let name = a.name();
            name.prefix().is_none() && CONDITIONAL_PROCESSING.contains(name.local_name().as_ref())
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
