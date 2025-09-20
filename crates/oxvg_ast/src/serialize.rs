//! Funcions for serializing XML trees
#[cfg(feature = "serialize")]
use std::io::Write;

#[cfg(feature = "serialize")]
pub use crate::error::XmlWriterError;
#[cfg(feature = "serialize")]
use crate::xmlwriter::XmlWriter;
#[cfg(feature = "serialize")]
pub use crate::xmlwriter::{Indent, Options};

use crate::error::PrinterError;

/// The destination to output serialized attribute and CSS values
pub type Printer<'a, 'b, 'c, W> = lightningcss::printer::Printer<'a, 'b, 'c, W>;
/// Options that control how attributes and CSS values are serialized
pub type PrinterOptions<'a> = lightningcss::printer::PrinterOptions<'a>;

/// Trait for values that can be serialized into string-like formats
pub trait ToAtom {
    /// Serialize `self` into SVG content, writing to `dest`
    ///
    /// # Errors
    /// If printer fails
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write;

    /// Serialize `self` into SVG content and return a string
    ///
    /// # Errors
    /// If writing string fails
    fn to_atom_string(&self, options: PrinterOptions) -> Result<String, PrinterError> {
        let mut s = String::new();
        let mut printer = Printer::new(&mut s, options);
        self.write_atom(&mut printer)?;
        Ok(s)
    }
}

impl<T> ToAtom for T
where
    T: lightningcss::traits::ToCss,
{
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.to_css(dest)
    }
}

#[cfg(feature = "serialize")]
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

#[cfg(feature = "serialize")]
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

        serialize_node(self, &mut xml)?;

        xml.end_document()
    }
}

#[cfg(feature = "serialize")]
fn serialize_node<'arena, W: Write>(
    node: crate::node::Ref<'_, 'arena>,
    xml: &mut XmlWriter<'arena, W>,
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
                    xml.write_text(&text)?;
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
        serialize_node(child, xml)?;
    }

    if is_element!(node) {
        xml.end_element()
    } else {
        Ok(())
    }
}
