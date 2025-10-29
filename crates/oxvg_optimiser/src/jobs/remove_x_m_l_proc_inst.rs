use oxvg_ast::{
    element::Element,
    node::Node,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Removes the xml declaration from the document.
///
/// # Correctness
///
/// This job may affect clients which expect XML (not SVG) and can't detect the MIME-type
/// as `image/svg+xml`
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveXMLProcInst(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveXMLProcInst {
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

    fn processing_instruction(
        &self,
        processing_instruction: &Node<'input, 'arena>,
        _context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if &*processing_instruction.node_name() == "xml" {
            processing_instruction.remove();
        }
        Ok(())
    }
}

impl Default for RemoveXMLProcInst {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_x_m_l_proc_inst() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeXmlProcInst": true }"#,
        Some(
            r#"<?xml version="1.0" standalone="no"?>
<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    // FIXME: Correctly retained, but serializer uses default PI
    insta::assert_snapshot!(test_config(
        r#"{ "removeXmlProcInst": true }"#,
        Some(
            r#"<?xml-stylesheet href="style.css" type="text/css"?>
<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    Ok(())
}
