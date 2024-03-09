mod attribute;
mod content;
mod tag;

use crate::{
    cursor::Cursor,
    diagnostics::SVGError,
    file_reader::FileReader,
    markup::{decoration, Decoration},
    Node,
};
use std::{cell::RefCell, iter::Peekable, rc::Rc};

pub use self::{
    attribute::{attributes, Attribute, Attributes},
    content::{content, NodeContent},
    tag::{tag_type, ETag, EmptyElemTag, STag, TagType},
};

// [3.0 Logical Structures](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-logical-struct)

#[derive(Debug)]
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
    file_reader: &mut FileReader,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<Element, Box<SVGError>> {
    file_reader.next();
    match file_reader.peek() {
        // [15], [19], [28]
        Some('!') => {
            file_reader.next();
            Ok(decoration(file_reader, Decoration::Decoration)?)
        }
        // [16], [23], [77]
        Some('?') => {
            file_reader.next();
            Ok(decoration(file_reader, Decoration::Declaration)?)
        }
        // [39], [40], [42], [43]
        Some(_) => Ok(tag_type(file_reader, parent)?),
        None => Ok(Element::EndOfFile),
    }
}

#[test]
fn test_element() {
    let mut tag = "<svg attr=\"hi\">";
    dbg!(element(&mut FileReader::new(tag), None));

    let mut decoration = "<!-- Hello, World! -->";
    dbg!(element(&mut FileReader::new(decoration), None));

    let mut declaration = "<!DOCTYPE html>";
    dbg!(element(&mut FileReader::new(declaration), None));

    let mut tag = "<hello>";
    dbg!(element(&mut FileReader::new(tag), None));
}
