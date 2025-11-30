use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
/// The method for ordering xmlns attributes
pub enum XMLNSOrder {
    /// Sort xmlns attributes alphabetically
    Alphabetical,
    #[default]
    /// Keep xmlns attributes at the front of the list
    Front,
    #[doc(hidden)]
    #[cfg(feature = "napi")]
    /// Compatibility option for NAPI
    // FIXME: force discriminated union to prevent NAPI from failing CI
    Napi(),
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// Sorts attributes into a predictable order.
///
/// This doesn't affect the size of a document but will likely improve readability
/// and compression of the document.
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
pub struct SortAttrs {
    /// A list of attributes in a given order.
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub order: Option<Vec<String>>,
    /// The method for ordering xmlns attributes
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub xmlns_order: Option<XMLNSOrder>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for SortAttrs {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let xmlns_order = self.xmlns_order.is_none() || self.xmlns_order == Some(XMLNSOrder::Front);
        match self.order.as_ref() {
            Some(order) => element.attributes().sort(order, xmlns_order),
            None => element.attributes().sort(DEFAULT_ORDER, xmlns_order),
        }

        Ok(())
    }
}

const DEFAULT_ORDER: &[&str] = &[
    "id", "width", "height", "x", "x1", "x2", "y", "y1", "y2", "cx", "cy", "r", "fill", "stroke",
    "marker", "d", "points",
];

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

    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": {} }"#,
        Some(
            r#"<svg xmlns:editor2="link2" fill="" b="" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:editor1="link1" xmlns="" d="">
    <!-- put xmlns and namespace attributes before others by default -->
    <rect editor2:b="" editor1:b="" editor2:a="" editor1:a="" />
</svg>"#
        ),
    )?);

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
