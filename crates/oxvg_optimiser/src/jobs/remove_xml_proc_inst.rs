use oxvg_ast::{
    element::Element,
    node::Node,
    visitor::{ContextFlags, Visitor},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::Job;

use super::PrepareOutcome;

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveXMLProcInst(bool);

impl<E: Element> Job<E> for RemoveXMLProcInst {
    fn prepare(
        &mut self,
        _document: &E::ParentChild,
        _context_flags: &ContextFlags,
    ) -> super::PrepareOutcome {
        if self.0 {
            PrepareOutcome::None
        } else {
            PrepareOutcome::Skip
        }
    }
}

impl<E: Element> Visitor<E> for RemoveXMLProcInst {
    type Error = String;

    fn processing_instruction(
        &mut self,
        processing_instruction: &mut <E as Node>::Child,
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
