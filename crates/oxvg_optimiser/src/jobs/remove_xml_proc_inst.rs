use oxvg_ast::{
    element::Element,
    node::{self, Node},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Job, JobDefault, PrepareOutcome};

use super::ContextFlags;

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveXMLProcInst(bool);

impl<E: Element> Job<E> for RemoveXMLProcInst {
    fn prepare(
        &mut self,
        document: &E::ParentChild,
        _context_flags: &ContextFlags,
    ) -> PrepareOutcome {
        if !self.0 {
            return PrepareOutcome::Skip;
        }

        for node in document.child_nodes_iter() {
            if node.node_type() != node::Type::ProcessingInstruction
                || node.node_name() != "xml".into()
            {
                continue;
            }
            node.remove();
            break;
        }
        PrepareOutcome::Skip
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
