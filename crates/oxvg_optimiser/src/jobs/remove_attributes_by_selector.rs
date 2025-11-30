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
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// A selector and set of attributes to remove.
pub struct Selector {
    /// A CSS selector.
    pub selector: String,
    /// A list of qualified attribute names.
    pub attributes: Vec<String>,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", serde(transparent))]
/// Removes attributes from elements that match a selector.
///
/// # Correctness
///
/// Removing attributes may visually change the document if they're
/// presentation attributes or selected with CSS.
///
/// # Errors
///
/// If the selector fails to parse.
pub struct RemoveAttributesBySelector(pub Vec<Selector>);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveAttributesBySelector {
    type Error = JobsError<'input>;

    fn document(
        &self,
        document: &Element<'input, 'arena>,
        _context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        for item in &self.0 {
            let selected = document
                .select(&item.selector)
                .map_err(|e| JobsError::InvalidUserSelector(format!("{e:#?}")))?;
            for element in selected {
                element
                    .attributes()
                    .retain(|attr| !item.attributes.contains(&attr.name().to_string()));
            }
        }

        Ok(())
    }
}

#[test]
fn remove_attributes_by_selector() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeAttributesBySelector": [{
            "selector": "[fill='#0f0']",
            "attributes": ["fill"]
        }] }"#,
        Some(
            r##"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <rect x="0" y="0" width="100" height="100" fill="#00ff00" stroke="#00ff00"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeAttributesBySelector": [{
            "selector": "[fill='#0f0']",
            "attributes": ["fill", "stroke"]
        }] }"#,
        Some(
            r##"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <rect x="0" y="0" width="100" height="100" fill="#00ff00" stroke="#00ff00"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r##"{ "removeAttributesBySelector": [
            {
                "selector": "[fill='#0f0']",
                "attributes": ["fill"]
            },
            {
                "selector": "#remove",
                "attributes": ["stroke", "id"]
            }
        ] }"##,
        Some(
            r##"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <rect id="remove" x="0" y="0" width="100" height="100" fill="#00ff00" stroke="#00ff00"/>
</svg>"##
        )
    )?);

    Ok(())
}
