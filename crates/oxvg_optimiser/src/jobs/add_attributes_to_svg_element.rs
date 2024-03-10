use oxvg_parser::Child;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Job;

#[derive(Serialize, Deserialize, Default)]
pub struct AddAttributesToSVGElement {
    attributes: HashMap<String, String>,
}

impl Job for AddAttributesToSVGElement {
    fn from_configuration(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    fn run(&self, node: &mut Child) {
        let Child::Element(element) = node else {
            return;
        };
        if element.name != "svg" {
            return;
        }

        for (key, value) in &self.attributes {
            if element.attributes.contains_key(key) {
                continue;
            }

            element.attributes.insert(key.clone(), value.clone());
        }
    }
}

#[test]
fn add_attributes_to_svg_element() -> Result<(), &'static str> {
    let document = oxvg_parser::Document::parse("<svg></svg>");
    let Some(element) = document.root.children.first() else {
        return Err("Failed to parse");
    };
    let child = &mut *element.borrow_mut();

    let job = &mut AddAttributesToSVGElement::default();
    job.attributes.insert("foo".into(), "bar".into());

    job.run(child);
    let Child::Element(element) = child else {
        return Err("Unexpected child type");
    };
    assert_eq!(
        element.attributes, job.attributes,
        "Should add new attribute"
    );

    job.attributes.insert("foo".into(), "baz".into());
    job.run(child);
    let Child::Element(element) = child else {
        return Err("Unexpected child type");
    };
    assert_ne!(
        element.attributes, job.attributes,
        "Should not overwrite existing attribute"
    );

    Ok(())
}
