// [2.4 Character Data and Markup](https://www.w3.org/TR/2006/REC-xml11-20060816/#syntax)

mod char_data;
mod decoration;
mod element;

use crate::{
    cursor::Cursor,
    diagnostics::SvgParseError,
    document::Node,
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

pub fn markup(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<(Cursor, Markup), Box<SvgParseError>> {
    match partial.peek() {
        Some('<') => {
            // start-tag, end-tag, empty-element-tag, comment, cdata, doctype, processing-data,
            // xml-declaration, text-declaration
            element(partial, cursor, parent).map(|(c, e)| (c, Markup::Element(e)))
        }
        Some(&c) if c == '&' || c == '%' => {
            // reference
            reference(partial, cursor).map(|(c, r)| (c, Markup::Reference(r)))
        }
        Some(_) => char_data(partial, cursor).map(|(c, s)| (c, Markup::CharData(s))),
        None => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    }
}

#[test]
fn test_markup() {
    let mut element = "<!-- Hello, world -->".chars().peekable();
    dbg!(markup(&mut element, Cursor::default(), None));

    let mut markup_example = "&amp;".chars().peekable();
    dbg!(markup(&mut markup_example, Cursor::default(), None));

    let mut char_data = "Hello, world".chars().peekable();
    dbg!(markup(&mut char_data, Cursor::default(), None));
}
