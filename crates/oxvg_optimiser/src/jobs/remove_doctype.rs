use oxvg_ast::{
    element::Element,
    node::Node,
    visitor::{ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Removes doctype definitions from the document.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveDoctype(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveDoctype {
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

    fn doctype(&self, doctype: &mut <E as Node<'arena>>::Child) -> Result<(), Self::Error> {
        doctype.remove();
        Ok(())
    }
}

impl Default for RemoveDoctype {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_doctype() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeDoctype": true }"#,
        Some(
            r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    Ok(())
}
