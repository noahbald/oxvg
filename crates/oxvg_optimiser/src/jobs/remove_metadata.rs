use oxvg_ast::{
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMetadata(bool);

impl<E: Element> Visitor<E> for RemoveMetadata {
    type Error = String;

    fn prepare(
        &mut self,
        _document: &E,
        _context_flags: &mut ContextFlags,
    ) -> super::PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        let name = element.qual_name();
        if name.prefix().is_some() {
            return Ok(());
        }

        if name.local_name().as_ref() == "metadata" {
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
