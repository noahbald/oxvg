mod attribute;
mod content;
mod tag;

use crate::{
    cursor::Cursor,
    diagnostics::SvgParseError,
    markup::{decoration, Decoration},
    Node,
};
use std::{cell::RefCell, iter::Peekable, rc::Rc};

pub use self::{
    attribute::{attributes, Attribute},
    content::{content, NodeContent},
    tag::{tag_type, ETag, EmptyElemTag, STag, TagType},
};

// [3.0 Logical Structures](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-logical-struct)

#[derive(PartialEq, Debug)]
pub enum Element {
    StartTag(STag),
    EndTag(ETag),
    EmptyTag(EmptyElemTag),
    Comment(String),
    CData(String),
    DocType(String),
    ProcessingInstructions(String),
    XMLDeclaration(String),
    EndOfFile,
}

pub fn element(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<(Cursor, Element), Box<SvgParseError>> {
    partial.next();
    match partial.next() {
        // [15], [19], [28]
        Some('!') => Ok(decoration(
            partial,
            cursor.advance(),
            Decoration::Decoration,
        )?),
        // [16], [23], [77]
        Some('?') => Ok(decoration(
            partial,
            cursor.advance(),
            Decoration::Declaration,
        )?),
        // [39], [40], [42], [43]
        Some(_) => Ok(tag_type(partial, cursor, parent)?),
        None => Ok((cursor, Element::EndOfFile)),
    }
}

#[test]
fn test_element() {
    let mut decoration = "<!-- Hello, World! -->".chars().peekable();
    dbg!(element(&mut decoration, Cursor::default(), None));

    let mut declaration = "<!DOCTYPE html>".chars().peekable();
    dbg!(element(&mut declaration, Cursor::default(), None));

    let mut tag = "<hello>".chars().peekable();
    dbg!(element(&mut tag, Cursor::default(), None));
}
