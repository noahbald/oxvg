use oxvg_ast::{
    element::Element,
    node::Node,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoveXMLProcInst(bool);

impl<E: Element> Visitor<E> for RemoveXMLProcInst {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn processing_instruction(
        &mut self,
        processing_instruction: &mut <E as Node>::Child,
        _context: &Context<E>,
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
