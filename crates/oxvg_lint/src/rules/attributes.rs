use super::Rule;
use rcdom::Node;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default)]
struct Rules {
    pub selector: String,
    pub whitelist: bool,
    pub order: Option<Order>,
    pub attributes: BTreeMap<String, Pattern>,
}

impl Rule for Rules {
    fn execute(&self, element: &Node) -> Vec<String> {
        if let Some(e) = self.order(element) {
            vec![e]
        } else {
            vec![]
        }
    }
}

impl Rules {
    pub fn order(&self, node: &Node) -> Option<String> {
        use rcdom::NodeData::Element;

        let Element { attrs, .. } = &node.data else {
            return None;
        };
        let attrs = &*attrs.borrow();
        let attrs: Vec<String> = attrs
            .iter()
            .map(|markup5ever::Attribute { name, .. }| match &name.prefix {
                Some(prefix) => format!("{prefix}:{}", name.local),
                None => name.local.to_string(),
            })
            .collect();
        let order: Vec<String> = match &self.order {
            Some(Order::Custom(order)) => order.clone(),
            Some(Order::Alphabetical) | None => {
                let mut order = attrs.clone();
                order.sort();
                order
            }
        };

        let mut positions: BTreeMap<&str, usize> = BTreeMap::new();
        order.iter().enumerate().for_each(|(i, attribute)| {
            positions.insert(attribute, i);
        });

        for pair in attrs.windows(2) {
            if positions.get(pair[0].as_str()) <= positions.get(pair[1].as_str()) {
                continue;
            }

            let found = &pair[1];
            return Some(format!(
                "Wrong ordering of attributes, found \"{found}\", expected \"{order:#?}\""
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
fn attributes_order() {
    use xml5ever::{
        driver::{parse_document, XmlParseOpts},
        tendril::TendrilSink,
    };

    let dom: rcdom::RcDom = parse_document(rcdom::RcDom::default(), XmlParseOpts::default())
        .one(r#"<svg z="" a=""></svg>"#);
    let root = &*dom.document.children.borrow()[0];

    // Expect some error, as "z" is before "a"
    let rule = Rules {
        order: Some(Order::Alphabetical),
        ..Rules::default()
    };
    assert!(rule.order(root).is_some());
}
