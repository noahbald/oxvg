// [2.4 Character Data and Markup](https://www.w3.org/TR/2006/REC-xml11-20060816/#syntax)

mod char_data;
mod decoration;
mod element;

use crate::{
    cursor::Cursor,
    diagnostics::SvgParseError,
    document::Node,
    file_reader::FileReader,
    references::{reference, Reference},
    SvgParseErrorMessage,
};
use std::{cell::RefCell, iter::Peekable, rc::Rc};

pub use self::{
    char_data::char_data,
    decoration::{decoration, Decoration},
    element::{
        attributes, content, element, Attribute, ETag, Element, EmptyElemTag, NodeContent, STag,
        TagType,
    },
};

#[derive(PartialEq, Debug)]
pub enum Markup {
    Element(Element),
    Reference(Reference),
    CharData(String),
}

/// Consumes the partial until it reaches the end of a piece of markup.
/// Markup can be:
/// - An element, starting with '<'; or
/// - A reference, starting with '&' or '%'; or
/// - Character data
///
/// # Errors
///
/// This function will return an error if the partial has ended
pub fn markup(
    file_reader: &mut FileReader,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<Markup, Box<SvgParseError>> {
    match file_reader.peek() {
        Some('<') => {
            // start-tag, end-tag, empty-element-tag, comment, cdata, doctype, processing-data,
            // xml-declaration, text-declaration
            element(file_reader, parent).map(|e| Markup::Element(e))
        }
        Some(&c) if c == '&' || c == '%' => {
            // reference
            reference(file_reader).map(|r| Markup::Reference(r))
        }
        Some(_) => char_data(file_reader).map(|s| Markup::CharData(s)),
        None => Ok(Markup::Element(Element::EndOfFile)),
    }
}

#[test]
fn test_markup() {
    let mut tag = "<svg attr=\"hi\">";
    assert!(matches!(
        markup(&mut FileReader::new(tag), None),
        Ok(Markup::Element(Element::StartTag(_)))
    ));

    let mut element = "<!-- Hello, world -->";
    dbg!(markup(&mut FileReader::new(element), None));

    let mut markup_example = "&amp;";
    dbg!(markup(&mut FileReader::new(markup_example), None));

    let mut char_data = "Hello, world";
    dbg!(markup(&mut FileReader::new(char_data), None));
}
