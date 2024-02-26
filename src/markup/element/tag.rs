// [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use crate::{
    characters::char,
    cursor::Cursor,
    diagnostics::SvgParseError,
    file_reader::FileReader,
    markup::{attributes, Attribute},
    syntactic_constructs::{whitespace, Name},
    Element, Node, Span, SvgParseErrorMessage,
};
use core::fmt;
use std::fmt::Display;
use std::{cell::RefCell, iter::Peekable, rc::Rc};

// [44]
#[derive(PartialEq, Debug)]
pub struct EmptyElemTag {
    parent: Option<Rc<RefCell<Node>>>,
    pub tag_name: Name,
    attributes: Vec<Attribute>,
    pub span: Span,
}

// [40]
#[derive(PartialEq, Default, Debug)]
pub struct STag {
    parent: Option<Rc<RefCell<Node>>>,
    pub tag_name: Name,
    attributes: Vec<Attribute>,
    pub span: Span,
}

impl STag {
    pub fn new(name: String, cursor: Cursor) -> Self {
        Self {
            parent: None,
            span: cursor.as_span((&name).len()),
            tag_name: name.into(),
            attributes: vec![],
        }
    }
}

// [42]
#[derive(PartialEq, Default, Debug)]
pub struct ETag {
    pub start_tag: Rc<RefCell<STag>>,
    pub tag_name: Name,
    pub span: Span,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TagType {
    SelfClosing,
    Any,
}

impl Display for TagType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let output = match self {
            TagType::SelfClosing => "<self-closing/>",
            TagType::Any => "<opening>, </closing>, or <self-closing />",
        };
        write!(f, "{:?}", output)
    }
}

/// Consumes the partial from name start char [\[4\]](https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-NameStartChar) until '>'
///
/// # Errors
///
/// This function will return an error if the tag is malformed
pub fn tag_type(
    file_reader: &mut FileReader,
    parent: Option<Rc<RefCell<Node>>>,
) -> Result<Element, Box<SvgParseError>> {
    let cursor_start = file_reader.get_cursor();
    if let Some('/') = file_reader.peek() {
        // [42]
        file_reader.next();
        let tag_name = Name::new(file_reader)?;
        let length = tag_name.len() + 2;
        whitespace(file_reader, false)?;
        char(file_reader, Some('>'))?;
        return Ok(Element::EndTag(ETag {
            start_tag: Rc::new(RefCell::new(STag::default())),
            tag_name,
            span: cursor_start.as_span(length),
        }));
    };

    let tag_name = Name::new(file_reader)?;
    match file_reader.peek() {
        Some('>') => {}
        Some('/') => {}
        _ => whitespace(file_reader, true)?,
    };
    let attributes = attributes(file_reader)?;

    match file_reader.next() {
        Some('/') => {
            // [44]
            char(file_reader, Some('>'))?;
            let length = tag_name.len() + 1;
            Ok(Element::EmptyTag(EmptyElemTag {
                parent,
                tag_name,
                attributes,
                span: cursor_start.as_span(length),
            }))
        }
        Some('>') => Ok(
            // [40]
            Element::StartTag(STag {
                parent,
                tag_name: tag_name.clone(),
                attributes,
                span: cursor_start.as_span(tag_name.len() + 1),
            }),
        ),
        Some(c) => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedChar(c, "> or />".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    }
}

#[test]
fn test_tag_type() {
    let mut open_tag = FileReader::new("svg attr=\"hi\">!</svg>");
    assert!(matches!(
        tag_type(&mut open_tag, None),
        Ok(Element::StartTag(_))
    ));
    assert_eq!(open_tag.next(), Some('!'));

    let mut empty_tag = FileReader::new("svg attr=\"hi\" />!");
    assert!(matches!(
        tag_type(&mut empty_tag, None),
        Ok(Element::EmptyTag(_))
    ));
    assert_eq!(open_tag.next(), Some('!'));

    let mut end_tag = FileReader::new("/svg>!");
    assert!(matches!(
        tag_type(&mut end_tag, None),
        Ok(Element::EndTag(_))
    ));
    assert_eq!(end_tag.next(), Some('!'));
}
