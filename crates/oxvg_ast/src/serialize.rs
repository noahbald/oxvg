//! Functions for serializing XML trees
use std::io::Write;

use oxvg_serialize::error::PrinterError;

use crate::error::XmlWriterError;
use crate::xmlwriter::XmlWriter;
pub use crate::xmlwriter::{Indent, Options, Space};

/// The destination to output serialized attribute and CSS values
pub type Printer<'a, 'b, 'c, W> = lightningcss::printer::Printer<'a, 'b, 'c, W>;
/// Options that control how attributes and CSS values are serialized
pub type PrinterOptions<'a> = lightningcss::printer::PrinterOptions<'a>;

/// Trait for values that can be serialized into string-like formats
pub trait ToValue {
    /// Serialize `self` into SVG content, writing to `dest`
    ///
    /// # Errors
    /// If printer fails
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write;

    /// Serialize `self` into SVG content and return a string
    ///
    /// # Errors
    /// If writing string fails
    fn to_value_string(&self, options: PrinterOptions) -> Result<String, PrinterError> {
        let mut s = String::new();
        let mut printer = Printer::new(&mut s, options);
        self.write_value(&mut printer)?;
        Ok(s)
    }
}

impl<T> ToValue for T
where
    T: lightningcss::traits::ToCss,
{
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.to_css(dest)
    }
}

/// An XML node serializer
pub trait Node<'input, 'arena> {
    /// # Errors
    /// If the underlying serialization fails
    fn serialize(&'arena self) -> Result<String, XmlWriterError> {
        self.serialize_with_options(Options::default())
    }

    /// # Errors
    /// If the serialization or write fails
    fn serialize_into<W: Write>(&'arena self, wr: W, options: Options)
        -> Result<W, XmlWriterError>;

    /// # Errors
    /// If the underlying serialization fails
    fn serialize_with_options(&'arena self, options: Options) -> Result<String, XmlWriterError>;
}

impl<'input, 'arena> Node<'input, 'arena> for crate::node::Node<'input, 'arena> {
    fn serialize_with_options(&'arena self, options: Options) -> Result<String, XmlWriterError> {
        let mut wr = Vec::new();
        self.serialize_into(&mut wr, options)?;
        String::from_utf8(wr).map_err(XmlWriterError::UTF8)
    }

    fn serialize_into<W: Write>(
        &'arena self,
        wr: W,
        options: Options,
    ) -> Result<W, XmlWriterError> {
        let mut xml = XmlWriter::new(wr, options);

        serialize_node(self, &mut xml, true, true)?;

        xml.end_document()
    }
}

fn serialize_node<'arena, W: Write>(
    node: crate::node::Ref<'_, 'arena>,
    xml: &mut XmlWriter<'arena, W>,
    is_first: bool,
    is_last: bool,
) -> Result<(), XmlWriterError> {
    use crate::{is_element, node};
    match node.node_type() {
        node::Type::Element => {
            let element = node.element().expect("type of element");
            xml.start_element(element.qual_name())?;
            for attr in element.attributes() {
                xml.write_attribute(&attr)?;
            }
        }
        node::Type::Text | node::Type::CDataSection => {
            if let Some(text) = node.text_content() {
                if !text.is_empty() {
                    xml.write_text(&text, is_first, is_last)?;
                }
            }
        }
        node::Type::Style => {
            if let Some(style) = node.style() {
                xml.write_style(&style.borrow())?;
            }
        }
        node::Type::Comment => xml.write_comment(&node.text_content().unwrap_or_default())?,
        node::Type::ProcessingInstruction => {
            let (target, value) = node.processing_instruction().expect("expected pi");
            xml.write_declaration(&target, &value)?;
        }
        node::Type::Document | node::Type::DocumentType | node::Type::DocumentFragment => {}
    }

    for child in node.child_nodes_iter() {
        serialize_node(
            child,
            xml,
            node.first_child().is_none_or(|n| n == child)
                || child
                    .previous_sibling()
                    .is_some_and(|n| n.node_type() == node::Type::Text),
            node.last_child().is_none_or(|n| n == child)
                || child
                    .next_sibling()
                    .is_some_and(|n| n.node_type() == node::Type::Text),
        )?;
    }

    if is_element!(node) {
        xml.end_element()
    } else {
        Ok(())
    }
}
