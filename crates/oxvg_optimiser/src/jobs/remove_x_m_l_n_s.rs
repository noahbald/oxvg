use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
/// Removes the `xmlns` attribute from `<svg>`.
///
/// This can be useful for SVGs that will be inlined.
///
/// # Correctness
///
/// This job may break document when used outside of HTML.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveXMLNS(#[napi(js_name = "enabled")] pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveXMLNS {
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
    ) -> Result<(), Self::Error> {
        if element.prefix().is_none() && element.local_name().as_ref() == "svg" {
            element.remove_attribute_local(&"xmlns".into());
            return Ok(());
        }

        Ok(())
    }
}

#[test]
fn remove_xmlns() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeXMLNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    Ok(())
}
