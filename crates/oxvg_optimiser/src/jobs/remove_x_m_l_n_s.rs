use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
pub struct RemoveXMLNS(bool);

impl<E: Element> Visitor<E> for RemoveXMLNS {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
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

    // FIXME: markup5ever adds xmlns after removal
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
