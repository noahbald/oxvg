use oxvg_ast::{Attributes, Child, Parent};
use serde::{Deserialize, Serialize};

use crate::Job;

#[derive(Serialize, Deserialize, Default)]
pub struct AddAttributesToSVGElement {
    attributes: Attributes,
}

impl Job for AddAttributesToSVGElement {
    fn from_configuration(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    fn run(&self, node: &mut Child) {
        let Child::Element(element) = node else {
            return;
        };
        let mut element = element.borrow_mut();
        if matches!(element.parent, Parent::Element(_)) {
            return;
        }
        if element.name().as_ref() != b"svg" {
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
    let document = oxvg_parser::FileReader::parse("<svg></svg>");
    let root = &*document.root.borrow();
    let Some(source_element) = root.children.first() else {
        return Err("Failed to parse");
    };
    let job = &mut AddAttributesToSVGElement::default();

    {
        let child = &mut *source_element.borrow_mut();

        job.attributes
            .insert(String::from("foo").into(), "bar".into());

        job.run(child);
        let Child::Element(element) = child else {
            return Err("Unexpected child type");
        };
        let element = &*element.borrow();
        assert_eq!(
            element.attributes, job.attributes,
            "Should add new attribute"
        );
    }

    {
        let child = &mut *source_element.borrow_mut();
        job.attributes
            .insert(String::from("foo").into(), "baz".into());
        job.run(child);
        let Child::Element(element) = child else {
            return Err("Unexpected child type");
        };
        let element = &*element.borrow();
        assert_ne!(
            element.attributes, job.attributes,
            "Should not overwrite existing attribute"
        );
    }

    Ok(())
}
