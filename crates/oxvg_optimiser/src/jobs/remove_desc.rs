use oxvg_ast::{
    element::Element,
    is_element,
    node::{self},
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
/// Removes the `<desc>` element from the document when empty or only contains editor attribution.
///
/// # Correctness
///
/// By default this job should never functionally change the document.
///
/// By using `remove_any` you may deteriotate the accessibility of the document for some users.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveDesc {
    #[serde(default = "bool::default")]
    /// Whether to remove all `<desc>` elements
    pub remove_any: bool,
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveDesc {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !is_element!(element, Desc) {
            return Ok(());
        }

        if self.remove_any
            || element.is_empty()
            || element.child_nodes_iter().any(|n| {
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
