use oxvg_ast::{
    element::Element,
    is_element, remove_attribute,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(transparent)]
/// Removes the `xmlns` attribute from `<svg>`.
///
/// This can be useful for SVGs that will be inlined.
///
/// # Correctness
///
/// This job may break document when used outside of HTML.
///
/// This may cause issues if the XMLNS doesn't reference the SVG namespace.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveXMLNS(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveXMLNS {
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
        if is_element!(element, Svg) {
            remove_attribute!(element, XMLNS);
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
