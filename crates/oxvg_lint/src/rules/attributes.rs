use super::Rule;
use oxvg_ast::{Child, Element};
use oxvg_diagnostics::SVGError;
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
        let element = &*element.borrow();

        let mut errors: Vec<SVGError> = Vec::new();
        if let Some(e) = self.order(element) {
            errors.push(e);
        }

        errors
    }
}

impl Rules {
    pub fn order(&self, element: &Element) -> Option<SVGError> {
        let attributes: Vec<String> = element
            .attributes
            .order
            .iter()
            .map(|key| String::from_utf8_lossy(key.into()).into())
            .collect();

        let order: Vec<String> = match &self.order {
            Some(Order::Custom(order)) => order.clone(),
            Some(Order::Alphabetical) | None => element
                .attributes
                .into_b_tree()
                .keys()
                .map(|key| String::from_utf8_lossy(key.into()).into())
                .collect(),
        };

        let mut positions: HashMap<&str, usize> = HashMap::new();
        order.iter().enumerate().for_each(|(i, attribute)| {
            positions.insert(attribute, i);
        });

        dbg!(&attributes);
        for pair in attributes.windows(2) {
            if positions.get(pair[0].as_str()) <= positions.get(pair[1].as_str()) {
                continue;
            }

            let found = &pair[1];
            return Some(SVGError::new(
                &format!(
                    "Wrong ordering of attributes, found \"{found}\", expected \"{order:#?}\""
                ),
                Some(element.span().into()),
            ));
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
fn attributes_order() -> Result<(), &'static str> {
    let document = oxvg_parser::FileReader::parse(r#"<svg z="" a=""></svg>"#);
    let root = &*document.root.borrow();
    let Some(element) = root.children.first() else {
        return Err("Failed to parse");
    };
    let Child::Element(element) = &*element.borrow() else {
        return Err("Unexpected child type");
    };
    let element = &*element.borrow();

    // Expect some error, as "z" before "a"
    let rule = Rules {
        order: Some(Order::Alphabetical),
        ..Rules::default()
    };
    assert!(rule.order(element).is_some());

    Ok(())
}
