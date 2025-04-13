//! Funcions for serializing XML trees
use std::io::Write;
use std::panic;

use xmlwriter::XmlWriter;

pub use xmlwriter::{Indent, Options};

use crate::attribute::{Attr as _, Attributes as _};
use crate::element::Element as _;
use crate::name::Name as _;
use crate::node;

/// A serialization error.
#[derive(Debug)]
pub enum Error {
    /// The serializer panicked while writing the dom to a string.
    SerializerPanicked,
    /// The serializer packicked while writing to a writer
    IO(std::io::Error),
}

/// An XML node serializer
pub trait Node<'arena> {
    /// # Errors
    /// If the underlying serialization fails
    fn serialize(&self) -> Result<String, Error> {
        self.serialize_with_options(Options::default())
    }

    /// # Errors
    /// If the serialization or write fails
    fn serialize_into<W: Write>(&self, mut wr: W, options: Options) -> Result<usize, Error> {
        wr.write(self.serialize_with_options(options)?.as_bytes())
            .map_err(Error::IO)
    }

    /// # Errors
    /// If the underlying serialization fails
    fn serialize_with_options(&self, options: Options) -> Result<String, Error>;
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "The serializer panicked while writing the dom to a string.".fmt(f)
    }
}

impl<'arena, T: node::Node<'arena>> Node<'arena> for T {
    fn serialize_with_options(&self, options: Options) -> Result<String, Error> {
        let mut xml = XmlWriter::new(options);

        serialize_node(self, &mut xml);

        panic::catch_unwind(|| xml.end_document()).map_err(|_| Error::SerializerPanicked)
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
            if let Some(text) = node.text_content() {
                let text = text.trim();
                if !text.is_empty() {
                    xml.write_text(text);
                }
            }
        }
        node::Type::Comment => {
            xml.write_comment(&node.text_content().unwrap_or_default());
        }
        node::Type::ProcessingInstruction => {
            // FIXME: Assuming all declarations are the same, since writer doesn't
            // support anything custom
            // let (target, value) = node.processing_instruction().expect("expected pi");
            xml.write_declaration();
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
