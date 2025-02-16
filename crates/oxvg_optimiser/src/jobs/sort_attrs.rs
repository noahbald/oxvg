use oxvg_ast::{
    attribute::Attributes,
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum XMLNSOrder {
    Alphabetical,
    #[default]
    Front,
}

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SortAttrs {
    pub order: Option<Vec<String>>,
    pub xmlns_order: Option<XMLNSOrder>,
}

impl<E: Element> Visitor<E> for SortAttrs {
    type Error = String;

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        let order = self.order.as_ref().unwrap_or_else(|| &DEFAULT_ORDER);
        let xmlns_order = self.xmlns_order.is_none() || self.xmlns_order == Some(XMLNSOrder::Front);
        element.attributes().sort(order, xmlns_order);

        Ok(())
    }
}

lazy_static! {
    pub static ref DEFAULT_ORDER: Vec<String> = vec![
        String::from("id"),
        String::from("width"),
        String::from("height"),
        String::from("x"),
        String::from("x1"),
        String::from("x2"),
        String::from("y"),
        String::from("y1"),
        String::from("y2"),
        String::from("cx"),
        String::from("cy"),
        String::from("r"),
        String::from("fill"),
        String::from("stroke"),
        String::from("marker"),
        String::from("d"),
        String::from("points"),
    ];
}

#[test]
fn sort_attrs() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": {} }"#,
        Some(
            r#"<svg r="" b="" x2="" cx="" y1="" a="" y="" y2="" x1="" cy="" x="">
    <!-- sort according to default list alphabetically -->
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": {} }"#,
        Some(
            r#"<svg a="" fill-opacity="" stroke="" fill="" stroke-opacity="">
    <!-- sort derived attributes like fill and fill-opacity -->
</svg>"#
        ),
    )?);

    // FIXME: rcdom is breaking this
    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": {} }"#,
        Some(
            r#"<svg xmlns:editor2="link" fill="" b="" xmlns:xlink="" xmlns:editor1="link" xmlns="" d="">
    <!-- put xmlns and namespace attributes before others by default -->
    <rect editor2:b="" editor1:b="" editor2:a="" editor1:a="" />
</svg>"#
        ),
    )?);

    // FIXME: rcdom is breaking this
    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": { "xmlnsOrder": "alphabetical" } }"#,
        Some(
            r#"<svg foo="bar" xmlns="http://www.w3.org/2000/svg" height="10" baz="quux" width="10" hello="world">
    <!-- optionally sort xmlns attributes alphabetically -->
    <rect x="0" y="0" width="100" height="100" stroke-width="1" stroke-linejoin="round" fill="red" stroke="orange" xmlns="http://www.w3.org/2000/svg"/>
    test
</svg>"#
        ),
    )?);

    Ok(())
}
