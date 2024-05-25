use std::rc::Rc;

use markup5ever::local_name;
use oxvg_ast::Attributes;
use oxvg_selectors::Element;
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default)]
pub struct CleanupEnableBackground {
    #[serde(skip_deserializing)]
    contains_filter: bool,
}

struct EnableBackgroundDimensions<'a> {
    width: &'a str,
    height: &'a str,
}

impl Job for CleanupEnableBackground {
    fn from_configuration(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap_or_default()
    }

    fn prepare(&mut self, document: &rcdom::RcDom) -> Option<()> {
        let root = &Element::from_document_root(document)?;
        self.prepare_contains_filter(root);
        None
    }

    /// Cleans up `enable-background`, unless document uses `<filter>` elements.
    ///
    /// Only cleans up attribute values
    /// TODO: Clean up inline-styles
    ///
    /// This job will:
    /// - Drop `enable-background` on `<svg>` node, if it matches the node's width and height
    /// - Set `enable-background` to `"new"` on `<mask>` or `<pattern>` nodes, if it matches the
    /// node's width and height
    fn run(&self, node: &Rc<rcdom::Node>) {
        use rcdom::NodeData::Element as ElementData;

        if self.contains_filter {
            return;
        };

        let ElementData { attrs, name, .. } = &node.data else {
            return;
        };
        let attrs = &mut *attrs.borrow_mut();
        let Some((enable_background_position, enable_background)) = attrs
            .iter()
            .enumerate()
            .find(|(_, attr)| attr.name.local == local_name!("enable-background"))
        else {
            return;
        };

        if name.local != local_name!("svg")
            && name.local != local_name!("mask")
            && name.local != local_name!("pattern")
        {
            attrs.remove(enable_background_position);
            return;
        }

        let enabled_background_dimensions =
            Self::get_enabled_background_dimensions(enable_background);
        let matches_dimensions =
            Self::enabled_background_matches(attrs, enabled_background_dimensions);
        if matches_dimensions && name.local != local_name!("svg") {
            if let Some(attr) = attrs.get_mut(enable_background_position) {
                attr.value = "new".into();
            }
        } else if name.local == local_name!("svg") {
            attrs.remove(enable_background_position);
        }
    }
}

impl CleanupEnableBackground {
    fn prepare_contains_filter(&mut self, root: &Element) {
        self.contains_filter = root.select("filter").unwrap().next().is_some();
    }

    fn get_enabled_background_dimensions(
        attr: &markup5ever::Attribute,
    ) -> Option<EnableBackgroundDimensions> {
        let markup5ever::Attribute { value, .. } = attr;
        let parameters: Vec<_> = value.split_whitespace().collect();
        // Only allow `new <x> <y> <width> <height>`
        if parameters.len() != 5 {
            return None;
        }

        Some(EnableBackgroundDimensions {
            width: parameters.get(3)?,
            height: parameters.get(4)?,
        })
    }

    fn enabled_background_matches(
        attrs: &Vec<markup5ever::Attribute>,
        dimensions: Option<EnableBackgroundDimensions>,
    ) -> bool {
        use markup5ever::tendril::Tendril;

        let Some(dimensions) = dimensions else {
            return false;
        };
        let attrs: Attributes = attrs.into();
        let Some(width) = attrs.get(&local_name!("width")) else {
            return false;
        };
        let Some(height) = attrs.get(&local_name!("height")) else {
            return false;
        };
        &Tendril::from(dimensions.width) == width && &Tendril::from(dimensions.height) == height
    }
}

#[test]
fn cleanup_enable_background() {
    use xml5ever::{
        driver::{parse_document, XmlParseOpts},
        tendril::TendrilSink,
    };

    let dom: rcdom::RcDom = parse_document(rcdom::RcDom::default(), XmlParseOpts::default())
        .one(r#"<svg width=".5" height="10" enable-background="new 0 0 .5 10"></svg>"#);
    let root = &dom.document.children.borrow()[0];
    let mut job = CleanupEnableBackground::default();

    job.prepare(&dom);
    job.run(root);

    assert_eq!(
        Element::new(root.clone()).get_attr(&local_name!("enable-background")),
        None
    );
}
