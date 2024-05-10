use markup5ever::{Attribute, QualName};
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default)]
pub struct AddClassesToSVG {
    pub class_names: Option<Vec<String>>,
    pub class_name: Option<String>,
}

impl Job for AddClassesToSVG {
    fn from_configuration(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    fn run(&self, node: &rcdom::Node) {
        use rcdom::NodeData::Element;

        if is_root(node) {
            return;
        }

        let Element { attrs, name, .. } = &node.data else {
            return;
        };
        if name.local.to_string() != "svg" {
            return;
        }

        let class_names = match self.class_names.clone() {
            Some(class_names) => class_names,
            None => match self.class_name.clone() {
                Some(class_name) => vec![class_name],
                None => return,
            },
        };
        let class_names = class_names.join(" ");

        let attrs = &mut *attrs.borrow_mut();
        let attr = attrs
            .iter_mut()
            .find(|attr| attr.name.local.to_string() == "class");
        let Some(attr) = attr else {
            attrs.push(Attribute {
                name: QualName::new(None, "".into(), "class".into()),
                value: class_names.into(),
            });
            return;
        };
        attr.value = format!("{} {}", attr.value.to_string().trim(), class_names).into();
    }
}

fn is_root(node: &rcdom::Node) -> bool {
    use rcdom::NodeData::Element;

    let parent_cell = node.parent.replace(None);
    let mut result = false;
    if let Some(parent) = &parent_cell {
        if let Some(parent) = parent.upgrade() {
            if let Element { .. } = parent.data {
                result = true;
            }
        };
        node.parent.replace(parent_cell);
    };
    result
}

#[test]
fn add_classes_to_svg() -> Result<(), &'static str> {
    use rcdom::NodeData::Element;
    use xml5ever::{
        driver::{parse_document, XmlParseOpts},
        tendril::TendrilSink,
    };

    let dom: rcdom::RcDom =
        parse_document(rcdom::RcDom::default(), XmlParseOpts::default()).one("<svg></svg>");
    let root = &*dom.document.children.borrow()[0];
    let job = AddClassesToSVG {
        class_names: Some(vec![String::from("foo"), String::from("bar")]),
        class_name: None,
    };

    dbg!(root);
    job.run(root);
    dbg!(root);
    let attrs = match &root.data {
        Element { attrs, .. } => attrs,
        _ => Err("Unexpected document structure")?,
    };
    let attrs = &*attrs.borrow();
    let Some(class) = attrs
        .iter()
        .find(|attr| attr.name.local.to_string() == "class")
    else {
        unreachable!("Class attribute missing");
    };
    assert_eq!(class.value, "foo bar".into());

    Ok(())
}
