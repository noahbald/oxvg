use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
/// Removes all `<style>` elements from the document.
///
/// # Correctness
///
/// This job is likely to visually affect documents with style elements in them.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveStyleElement(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveStyleElement {
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
