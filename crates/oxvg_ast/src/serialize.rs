//! Funcions for serializing XML trees
use std::panic;

use xmlwriter::XmlWriter;

pub use xmlwriter::Options;

use crate::attribute::{Attr as _, Attributes as _};
use crate::element::Element as _;
use crate::name::Name as _;
use crate::node;

/// An XML node serializer
pub trait Node<'arena> {
    /// # Errors
    /// If the underlying serialization fails
    fn serialize(&self) -> std::thread::Result<String> {
        self.serialize_with_options(Options::default())
    }

    /// # Errors
    /// If the underlying serialization fails
    fn serialize_with_options(&self, options: Options) -> std::thread::Result<String>;
}

impl<'arena, T: node::Node<'arena>> Node<'arena> for T {
    fn serialize_with_options(&self, options: Options) -> std::thread::Result<String> {
        let mut xml = XmlWriter::new(options);

        serialize_node(self, &mut xml);

        panic::catch_unwind(|| xml.end_document())
    }
}

fn serialize_node<'arena, T: node::Node<'arena>>(node: &T, xml: &mut XmlWriter) {
    match node.node_type() {
        node::Type::Element => {
            let element = node.element().expect("type of element");
            element.qual_name().with_str(|s| xml.start_element(s));
            for attr in element.attributes().into_iter() {
                attr.name()
                    .with_str(|s| xml.write_attribute(s, attr.value()));
            }
        }
        node::Type::Text => {
            xml.write_text(&node.text_content().unwrap_or_default());
        }
        node::Type::Comment => {
            xml.write_comment(&node.text_content().unwrap_or_default());
        }
        _ => {}
    }

    for child in node.child_nodes_iter() {
        serialize_node(&child, xml);
    }

    if node.node_type() == node::Type::Element {
        xml.end_element();
    }
}
