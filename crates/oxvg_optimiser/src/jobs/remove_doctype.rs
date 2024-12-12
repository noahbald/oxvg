use oxvg_ast::{
    element::Element,
    node::{self, Node},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Job, JobDefault, PrepareOutcome};

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveDoctype(bool);

impl<E: Element> Job<E> for RemoveDoctype {
    fn prepare(&mut self, document: &E::ParentChild) -> PrepareOutcome {
        if !self.0 {
            return PrepareOutcome::Skip;
        }

        for node in document.child_nodes_iter() {
            if node.node_type() != node::Type::DocumentType {
                continue;
            }
            node.remove();
            break;
        }
        PrepareOutcome::Skip
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
