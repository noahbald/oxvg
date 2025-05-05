use oxvg_ast::{
    element::Element,
    node::Node,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
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
pub struct RemoveXMLProcInst(#[napi(js_name = "enabled")] pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveXMLProcInst {
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

    fn processing_instruction(
        &self,
        processing_instruction: &mut <E as Node<'arena>>::Child,
        _context: &Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if processing_instruction.node_name() == "xml".into() {
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
fn remove_xml_proc_inst() -> anyhow::Result<()> {
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
