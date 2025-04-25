use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
/// Removes the `<title>` element from the document.
///
/// This may affect the accessibility of documents, where the title is used
/// to describe a non-decorative SVG.
///
/// # Correctness
///
/// This job may visually change documents with images inlined in them.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveTitle(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveTitle {
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
