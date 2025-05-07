use oxvg_ast::{
    attribute::Attr,
    element::Element,
    name::Name,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
/// Removes empty `<text>` and `<tspan>` elements. Removes `<tref>` elements that don't
/// reference anything within the document.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveEmptyText {
    /// Whether to remove empty text elements.
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub text: Option<bool>,
    /// Whether to remove empty tspan elements.
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub tspan: Option<bool>,
    /// Whether to remove useless tref elements.
    ///
    /// `tref` is deprecated and generally unsupported by browsers.
    #[cfg_attr(feature = "wasm", tsify(optional))]
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
