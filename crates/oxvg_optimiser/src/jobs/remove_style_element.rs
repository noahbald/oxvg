use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct RemoveStyleElement(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveStyleElement {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(
        &mut self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        if element.prefix().is_none() && element.local_name().as_ref() == "style" {
            element.remove();
            return Ok(());
        }

        Ok(())
    }
}

#[test]
fn remove_style_element() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeStyleElement": true }"#,
        Some(
            r#"<svg version="1.1" id="Layer_1" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" viewBox="0 0 100 100" style="enable-background:new 0 0 100 100;" xml:space="preserve">
    <style type="text/css">
    .st0 {
        fill: #231F20;
    }
    </style>
    <circle class="st0" cx="50" cy="50" r="50" />
</svg>"#
        ),
    )?);

    Ok(())
}
