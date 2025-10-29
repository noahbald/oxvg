use oxvg_ast::{
    attribute::data::AttrId,
    element::Element,
    is_element,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
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
        _info: &Info<'input, 'arena>,
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
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if is_element!(element, Svg) {
            element.remove_attribute(&AttrId::XMLNS);
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
