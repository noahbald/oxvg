use oxvg_ast::{
    element::Element,
    has_attribute, is_element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

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

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveEmptyText {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if self.text.unwrap_or(true) && is_element!(element, Text) && element.is_empty() {
            element.remove();
        }

        if self.tspan.unwrap_or(true) && is_element!(element, TSpan) && element.is_empty() {
            element.remove();
        }

        if self.tref.unwrap_or(true)
            && is_element!(element, TRef)
            && !has_attribute!(element, XLinkHref)
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
