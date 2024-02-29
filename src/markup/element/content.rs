// [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use crate::{
    cursor::Cursor, diagnostics::SvgParseError, document::node, file_reader::FileReader,
    markup::markup, ETag, Element, Markup, Node, STag, SvgParseErrorMessage,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub enum NodeContent {
    Element(Element),
    Node(Rc<RefCell<Node>>),
    Markup(Markup),
}

pub fn content(
    file_reader: &mut FileReader,
    parent: Rc<RefCell<Node>>,
) -> Result<(Vec<NodeContent>, ETag), Box<SvgParseError>> {
    // [43]
    let mut content = Vec::new();
    let tag_name = match &*parent.borrow() {
        Node::ContentNode((start_tag, ..)) => start_tag.borrow().tag_name.clone(),
        Node::EmptyNode(_) => {
            unreachable!(
                "Error: Attempted to parse content of empty tag. Please raise a bugfix request"
            )
        }
    };
    loop {
        let item = markup(file_reader, Some(Rc::clone(&parent)))?;
        match item {
            Markup::Element(e) => match e {
                Element::StartTag(t) => {
                    let node = node(file_reader, Rc::new(RefCell::new(t)))?;
                    content.push(NodeContent::Node(node));
                }
                Element::EmptyTag(t) => {
                    content.push(NodeContent::Node(Rc::new(RefCell::new(Node::EmptyNode(t)))))
                }
                Element::EndTag(t) if t.tag_name == tag_name => {
                    return Ok((content, t));
                }
                Element::EndTag(t) => Err(SvgParseError::new_span(
                    t.span,
                    SvgParseErrorMessage::UnmatchedTag(t.tag_name.into(), tag_name.clone().into()),
                ))?,
                Element::EndOfFile => Err(SvgParseError::new_curse(
                    file_reader.get_cursor(),
                    SvgParseErrorMessage::ExpectedEndOfFile,
                ))?,
                e => content.push(NodeContent::Element(e)),
            },
            m => content.push(NodeContent::Markup(m)),
        }
    }
}

#[test]
fn test_content() {
    let start_tag = Rc::new(RefCell::new(STag::new("e".into(), Cursor::default())));
    let parent = Rc::new(RefCell::new(Node::ContentNode((
        start_tag,
        Vec::new(),
        ETag::default(),
    ))));
    let mut element = FileReader::new("<!-- Hello, world --></e>");
    dbg!(content(&mut element, Rc::clone(&parent)),);

    let mut node = FileReader::new("<example />");
    dbg!(content(&mut node, Rc::clone(&parent)));

    let mut markup = FileReader::new("&lt;_&gt;");
    dbg!(content(&mut markup, Rc::clone(&parent)));
}
