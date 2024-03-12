use super::Rule;
use oxvg_parser::{Child, Element, SVGError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default)]
struct Rules {
    pub selector: String,
    pub whitelist: bool,
    pub order: Option<Order>,
    pub attributes: HashMap<String, Pattern>,
}

impl Rule for Rules {
    fn execute(&self, element: Child) -> Vec<SVGError> {
        let Child::Element(element) = element else {
            return vec![];
        };

        let mut errors: Vec<SVGError> = Vec::new();
        if let Some(e) = self.order(&element) {
            errors.push(e);
        }

        errors
    }
}

impl Rules {
    pub fn order(&self, element: &Element) -> Option<SVGError> {
        let attributes = &element.attributes_order;

        let order = if let Some(Order::Custom(order)) = self.order.clone() {
            order
        } else {
            let mut order = attributes.clone();
            order.sort_unstable();
            order
        };

        let mut positions: HashMap<&str, usize> = HashMap::new();
        order.iter().enumerate().for_each(|(i, attribute)| {
            positions.insert(attribute, i);
        });

        dbg!(&attributes);
        for pair in attributes.windows(2) {
            if positions.get(&*pair[0]) <= positions.get(&*pair[1]) {
                continue;
            }

            return Some(SVGError::new(format!("Wrong ordering of attributes, found \"{attributes:?}\", expected \"{order:?}\""), (0..0).into()));
        }
        None
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum Order {
    Alphabetical,
    Custom(Vec<String>),
}

#[derive(Serialize, Deserialize)]
enum Pattern {
    Required,
    Exact(String),
    OneOf(Vec<String>),
    Match(String),
    Optional(Box<Pattern>),
}

#[test]
fn attributes() -> Result<(), &'static str> {
    let document = oxvg_parser::Document::parse("<svg z a></svg>");
    let root = &*document.root.borrow();
    let Some(element) = root.children.first() else {
        return Err("Failed to parse");
    };
    let Child::Element(element) = &*element.borrow() else {
        return Err("Unexpected child type");
    };

    // Expect some error, as "z" before "a"
    let rule = Rules {
        order: Some(Order::Alphabetical),
        ..Rules::default()
    };
    assert!(rule.order(element).is_some());

    Ok(())
}
