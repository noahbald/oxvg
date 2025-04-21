//! Funcions for serializing XML trees
use std::io::Write;

use crate::xmlwriter::{Error, XmlWriter};
pub use crate::xmlwriter::{Indent, Options};

use crate::attribute::{Attr as _, Attributes as _};
use crate::element::Element as _;
use crate::name::Name as _;
use crate::node;

/// An XML node serializer
pub trait Node<'arena> {
    /// # Errors
    /// If the underlying serialization fails
    fn serialize(&self) -> Result<String, Error> {
        self.serialize_with_options(Options::default())
    }

    /// # Errors
    /// If the serialization or write fails
    fn serialize_into<W: Write>(&self, wr: W, options: Options) -> Result<W, Error>;

    /// # Errors
    /// If the underlying serialization fails
    fn serialize_with_options(&self, options: Options) -> Result<String, Error>;
}

impl<'arena, T: node::Node<'arena, Child = T>> Node<'arena> for T {
    fn serialize_with_options(&self, options: Options) -> Result<String, Error> {
        let mut wr = Vec::new();
        self.serialize_into(&mut wr, options)?;
        String::from_utf8(wr).map_err(Error::UTF8)
    }

    fn serialize_into<W: Write>(&self, wr: W, options: Options) -> Result<W, Error> {
        let mut xml = XmlWriter::new(wr, options);

        serialize_node(self, &mut xml)?;

        xml.end_document()
    }
}

fn serialize_node<'arena, T: node::Node<'arena, Child = T>, W: Write>(
    node: &T,
    xml: &mut XmlWriter<W, T::Name>,
) -> Result<(), Error> {
    match node.node_type() {
        node::Type::Element => {
            let element = node.element().expect("type of element");
            xml.start_element(element.qual_name().clone())?;
            for attr in element.attributes().into_iter() {
                attr.name()
                    .with_str(|s| xml.write_attribute(s, attr.value()))?;
            }
        }
        node::Type::Text => {
            if let Some(text) = node.text_content() {
                let text = text.trim();
                if !text.is_empty() {
                    xml.write_text(text)?;
                }
            }
        }
        node::Type::Comment => {
            xml.write_comment(&node.text_content().unwrap_or_default())?;
        }
        node::Type::ProcessingInstruction => {
            let (target, value) = node.processing_instruction().expect("expected pi");
            xml.write_declaration(&target, &value)?;
        }
        _ => {}
    }

    for child in node.child_nodes_iter() {
        serialize_node(&child, xml)?;
    }

    if node.node_type() == node::Type::Element {
        xml.end_element()
    } else {
        Ok(())
    }
}
