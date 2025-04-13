use oxvg_ast::{
    element::Element,
    node::Node,
    visitor::{ContextFlags, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoveDoctype(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveDoctype {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn doctype(&mut self, doctype: &mut <E as Node<'arena>>::Child) -> Result<(), Self::Error> {
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
