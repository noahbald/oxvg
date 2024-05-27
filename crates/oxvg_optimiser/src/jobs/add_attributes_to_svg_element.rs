use std::{collections::HashSet, rc::Rc};

use oxvg_ast::Attributes;
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default, Clone)]
pub struct AddAttributesToSVGElement {
    pub attributes: Attributes,
}

impl Job for AddAttributesToSVGElement {
    fn run(&self, node: &Rc<rcdom::Node>) {
        use rcdom::NodeData::Element;

        let Element { attrs, .. } = &node.data else {
            return;
        };
        let attrs = &mut *attrs.borrow_mut();
        let keys: HashSet<_> = attrs.iter().map(|attr| attr.name.clone()).collect();

        for attr in &Into::<Vec<markup5ever::Attribute>>::into(&self.attributes) {
            let key = &attr.name;
            if keys.contains(key) {
                continue;
            }
            attrs.push(attr.clone());
        }
    }
}

#[test]
fn add_attributes_to_svg_element() -> Result<(), &'static str> {
    use html5ever::{tendril::TendrilSink, ParseOpts};
    use rcdom::NodeData::Element;

    let dom: rcdom::RcDom =
        html5ever::parse_document(rcdom::RcDom::default(), ParseOpts::default()).one("<svg></svg>");
    let root = &dom.document.children.borrow()[0];
    let job = &mut AddAttributesToSVGElement::default();

    job.attributes
        .insert(markup5ever::LocalName::from("foo"), "bar".into());
    job.run(root);
    match &root.data {
        Element { attrs, .. } => {
            assert_eq!(
                attrs.borrow().last(),
                Some(&markup5ever::Attribute {
                    name: markup5ever::QualName::new(None, ns!(svg), "foo".into()),
                    value: "bar".into()
                })
            );
        }
        _ => Err("Attribute not added")?,
    }

    job.attributes
        .insert(markup5ever::LocalName::from("foo"), "baz".into());
    job.run(root);
    match &root.data {
        Element { attrs, .. } => {
            assert_eq!(
                attrs.borrow().last(),
                Some(&markup5ever::Attribute {
                    name: markup5ever::QualName::new(None, ns!(svg), "foo".into()),
                    value: "bar".into()
                })
            );
        }
        _ => Err("Attribute not added")?,
    }

    Ok(())
}
