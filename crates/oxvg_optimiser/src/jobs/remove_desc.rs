use oxvg_ast::{
    element::Element,
    node::{self, Node},
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct RemoveDesc {
    pub remove_any: Option<bool>,
}

impl<E: Element> Visitor<E> for RemoveDesc {
    type Error = String;

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        if element.prefix().is_some() || element.local_name().as_ref() != "desc" {
            return Ok(());
        }

        if self.remove_any.unwrap_or(false)
            || element.is_empty()
            || element.any_child(|n| {
                n.node_type() == node::Type::Text
                    && n.text_content()
                        .is_some_and(|s| STANDARD_DESCS.is_match(&s))
            })
        {
            element.remove();
        }

        Ok(())
    }
}

lazy_static! {
    static ref STANDARD_DESCS: regex::Regex = regex::Regex::new("^Created (with|using)").unwrap();
}

#[test]
fn remove_desc() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeDesc": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <desc>Created with Sketch.</desc>
    <g/>
</svg>"#
        ),
    )?);

    Ok(())
}
