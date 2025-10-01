use oxvg_ast::{
    element::Element,
    is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
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

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveTitle {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if is_element!(element, Title) {
            element.remove();
        }

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
