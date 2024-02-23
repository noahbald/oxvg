// [3.1 Start-Tags, End-Tags, and Empty-Element Tags](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-starttags)

use crate::{
    cursor::Cursor, diagnostics::SvgParseError, document::node, markup::markup, ETag, Element,
    Markup, Node, STag, SvgParseErrorMessage,
};
use std::{cell::RefCell, iter::Peekable, rc::Rc};

#[derive(PartialEq, Debug)]
pub enum NodeContent {
    Element(Element),
    Node(Rc<RefCell<Node>>),
    Markup(Markup),
}

pub fn content(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    parent: Rc<RefCell<Node>>,
) -> Result<(Cursor, Vec<NodeContent>, ETag), Box<SvgParseError>> {
    // [43]
    let mut cursor = cursor;
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
        let (c, item) = markup(partial, cursor, Some(Rc::clone(&parent)))?;
        cursor = c;
        match item {
            Markup::Element(e) => match e {
                Element::StartTag(t) => {
                    let (c, node) = node(partial, cursor, Rc::new(RefCell::new(t)))?;
                    cursor = c;
                    content.push(NodeContent::Node(node));
                }
                Element::EmptyTag(t) => {
                    content.push(NodeContent::Node(Rc::new(RefCell::new(Node::EmptyNode(t)))))
                }
                Element::EndTag(t) if t.tag_name == tag_name => {
                    return Ok((cursor, content, t));
                }
                Element::EndTag(t) => Err(SvgParseError::new_span(
                    t.span,
                    SvgParseErrorMessage::UnmatchedTag(t.tag_name.into(), tag_name.clone().into()),
                ))?,
                Element::EndOfFile => Err(SvgParseError::new_curse(
                    cursor,
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
    let mut element = "<!-- Hello, world --></e>".chars().peekable();
    assert_eq!(
        content(&mut element, Cursor::default(), Rc::clone(&parent)),
        Ok((
            Cursor::default().advance_by(21),
            vec![NodeContent::Element(Element::Comment(
                "<!-- Hello, world -->".into()
            ))],
            ETag::default()
        ))
    );

    let mut node = "<example />".chars().peekable();
    dbg!(content(&mut node, Cursor::default(), Rc::clone(&parent)));

    let mut markup = "&lt;_&gt;".chars().peekable();
    dbg!(content(&mut markup, Cursor::default(), Rc::clone(&parent)));
}
