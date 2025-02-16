use oxvg_ast::{
    element::Element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RemoveTitle(bool);

impl<E: Element> Visitor<E> for RemoveTitle {
    type Error = String;

    fn prepare(
        &mut self,
        _document: &E,
        _context_flags: &mut oxvg_ast::visitor::ContextFlags,
    ) -> oxvg_ast::visitor::PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        if element.prefix().is_some() || element.local_name().as_ref() != "title" {
            return Ok(());
        }
        element.remove();

        Ok(())
    }
}

impl Default for RemoveTitle {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_title() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeTitle": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <title>...</title>
    <g/>
</svg>"#
        ),
    )?);

    Ok(())
}
