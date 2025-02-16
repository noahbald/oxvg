use oxvg_ast::{
    element::Element,
    name::Name,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Selector {
    selector: String,
    attributes: Vec<String>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAttributesBySelector(pub Vec<Selector>);

impl<E: Element> Visitor<E> for RemoveAttributesBySelector {
    type Error = String;

    fn document(&mut self, document: &mut E, _context: &Context<E>) -> Result<(), Self::Error> {
        for item in &self.0 {
            let selected = match document.select(&item.selector) {
                Ok(iter) => iter,
                Err(error) => return Err(format!("{error:?}")),
            };
            let attribute_names: Vec<_> = item
                .attributes
                .iter()
                .map(String::as_str)
                .map(E::Name::parse)
                .collect();
            for element in selected {
                for attr in &attribute_names {
                    element.remove_attribute(attr);
                }
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
            "selector": "[fill='#00ff00']",
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
            "selector": "[fill='#00ff00']",
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
                "selector": "[fill='#00ff00']",
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
