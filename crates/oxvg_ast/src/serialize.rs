//! Funcions for serializing XML trees
use std::io::Write;

#[cfg(feature = "serialize")]
use crate::xmlwriter::XmlWriter;
#[cfg(feature = "serialize")]
pub use crate::xmlwriter::{Indent, Options};
use crate::{
    error::{PrinterError, XmlWriterError},
    node::Ref,
};

use crate::node;

pub type Printer<'a, 'b, 'c, W> = lightningcss::printer::Printer<'a, 'b, 'c, W>;
pub type PrinterOptions<'a> = lightningcss::printer::PrinterOptions<'a>;

pub trait ToAtom {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write;

    fn to_atom_string(&self, options: PrinterOptions) -> Result<String, PrinterError> {
        let mut s = String::new();
        let mut printer = Printer::new(&mut s, options);
        self.write_atom(&mut printer)?;
        Ok(s.into())
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
pub trait Node<'arena> {
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
impl<'arena> Node<'arena> for crate::node::Node<'arena> {
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
    node: Ref<'arena>,
    xml: &mut XmlWriter<'arena, W>,
) -> Result<(), XmlWriterError> {
    match node.node_type() {
        node::Type::Element => {
            let element = node.element().expect("type of element");
            xml.start_element(element.qual_name())?;
            for attr in element.attributes().into_iter() {
                xml.write_attribute(&*attr)?;
            }
        }
        node::Type::Text => {
            if let Some(text) = node.text_content() {
                if !text.is_empty() {
                    xml.write_text(&text)?
                }
            }
        }
        node::Type::Comment => xml.write_comment(&node.text_content().unwrap_or_default())?,
        node::Type::ProcessingInstruction => {
            let (target, value) = node.processing_instruction().expect("expected pi");
            xml.write_declaration(&target, &value)?
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
