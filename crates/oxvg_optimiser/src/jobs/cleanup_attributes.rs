use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default)]
pub struct CleanupAttributes {
    newlines: Option<bool>,
    trim: Option<bool>,
    spaces: Option<bool>,
}

impl Job for CleanupAttributes {
    fn from_configuration(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    fn run(&self, node: &rcdom::Node) {
        use rcdom::NodeData::Element;

        let Element { attrs, .. } = &node.data else {
            return;
        };
        let attrs = &mut *attrs.borrow_mut();

        for attr in attrs.iter_mut() {
            if matches!(self.newlines, Some(true)) {
                attr.value = attr.value.replace('\n', " ").into();
            }
            if matches!(self.trim, Some(true)) {
                attr.value = attr.value.trim().into();
            }
            if matches!(self.spaces, Some(true)) {
                attr.value = attr
                    .value
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .into();
            }
        }
    }
}

#[test]
fn cleanup_attributes() -> Result<(), &'static str> {
    use rcdom::NodeData::Element;
    use xml5ever::{
        driver::{parse_document, XmlParseOpts},
        tendril::TendrilSink,
    };

    let dom: rcdom::RcDom = parse_document(rcdom::RcDom::default(), XmlParseOpts::default()).one(
        r#"<svg class="  foo  bar 
        baz"></svg>"#,
    );
    let root = &*dom.document.children.borrow()[0];
    let job = CleanupAttributes {
        newlines: Some(true),
        trim: Some(true),
        spaces: Some(true),
    };

    job.run(root);
    let attrs = match &root.data {
        Element { attrs, .. } => attrs,
        _ => Err("Unexpected document structure")?,
    };
    let attrs = &*attrs.borrow();
    assert_eq!(
        attrs.first().map(|attr| &attr.value),
        Some(&"foo bar baz".into())
    );

    Ok(())
}
