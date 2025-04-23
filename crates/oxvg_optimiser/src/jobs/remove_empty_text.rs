use oxvg_ast::{
    attribute::Attr,
    element::Element,
    name::Name,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveEmptyText {
    pub text: Option<bool>,
    pub tspan: Option<bool>,
    pub tref: Option<bool>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveEmptyText {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        let name = element.qual_name().formatter().to_string();

        if self.text.unwrap_or(true) && &name == "text" && element.is_empty() {
            element.remove();
        }

        if self.tspan.unwrap_or(true) && &name == "tspan" && element.is_empty() {
            element.remove();
        }

        let xlink_name = <E::Attr as Attr>::Name::new(Some("xlink".into()), "href".into());
        if self.tref.unwrap_or(true) && &name == "tref" && !element.has_attribute(&xlink_name) {
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
