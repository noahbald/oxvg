use oxvg_ast::{
    element::Element,
    name::Name,
    visitor::{ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMetadata(bool);

impl<E: Element> Visitor<E> for RemoveMetadata {
    type Error = String;

    fn prepare(&mut self, _document: &E,  _context_flags: &mut ContextFlags) -> super::PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &super::Context<E>) -> Result<(), String> {
        let name = element.qual_name();
        if name.prefix().is_some() {
            return Ok(());
        }

        if name.local_name() == "metadata".into() {
            element.remove();
        }
        Ok(())
    }
}

impl Default for RemoveMetadata {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_metadata() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeMetadata": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <metadata>...</metadata>
    <g/>
</svg>"#
        ),
    )?);

    Ok(())
}
