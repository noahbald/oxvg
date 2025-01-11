use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveEmptyText {
    text: Option<bool>,
    tspan: Option<bool>,
    tref: Option<bool>,
}

impl<E: Element> Visitor<E> for RemoveEmptyText {
    type Error = String;

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), Self::Error> {
        let name = element.qual_name().to_string();

        if self.text.unwrap_or(true) && &name == "text" && element.is_empty() {
            element.remove();
        }

        if self.tspan.unwrap_or(true) && &name == "tspan" && element.is_empty() {
            element.remove();
        }

        if self.tref.unwrap_or(true)
            && &name == "tref"
            && !element.has_attribute(&"xlink:href".into())
        {
            element.remove();
        }

        Ok(())
    }
}

#[test]
fn remove_empty_text() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyText": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove empty text -->
    <g>
        <text></text>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyText": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove empty tspan -->
    <g>
        <tspan></tspan>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEmptyText": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove non-linking tref -->
    <g>
        <tref>...</tref>
    </g>
</svg>"#
        ),
    )?);

    Ok(())
}
