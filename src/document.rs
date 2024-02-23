// [2.1 Well-Formed XML Documents](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-well-formed)
use crate::{
    content, cursor::Cursor, diagnostics::SvgParseError, markup, ETag, Element, EmptyElemTag,
    Markup, NodeContent, STag, SvgParseErrorMessage, TagType,
};
use std::{cell::RefCell, iter::Peekable, rc::Rc};

#[derive(Debug)]
pub struct Document {
    pub prolog: Vec<Markup>,
    pub element: Rc<RefCell<Node>>,
    pub misc: Vec<Markup>,
}

impl Document {
    pub fn new(
        partial: &mut Peekable<impl Iterator<Item = char>>,
        cursor: Cursor,
    ) -> Result<(Cursor, Self), Box<SvgParseError>> {
        // [Document](https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-document)
        // [1]
        let mut cursor = cursor;
        let mut prolog = Vec::new();
        let root_start = Rc::new(RefCell::new(STag::default()));
        loop {
            let (c, item) = markup(partial, cursor, None)?;
            cursor = c;
            match item {
                Markup::Element(e) => match e {
                    Element::StartTag(e) => {
                        root_start.replace(e);
                        break;
                    }
                    Element::EmptyTag(EmptyElemTag { span, .. })
                    | Element::EndTag(ETag { span, .. }) => {
                        Err(SvgParseError::new_span(
                            span,
                            SvgParseErrorMessage::UnexpectedTagType(TagType::SelfClosing),
                        ))?;
                    }
                    e => prolog.push(Markup::Element(e)),
                },
                m => prolog.push(m),
            };
        }
        let (mut cursor, element) = node(partial, cursor, root_start)?;

        let mut misc = Vec::new();
        loop {
            let (c, item) = markup(partial, cursor, None)?;
            cursor = c;
            match item {
                Markup::Element(e) => match e {
                    Element::EndOfFile => {
                        return Ok((
                            cursor,
                            Document {
                                prolog,
                                element,
                                misc,
                            },
                        ))
                    }
                    Element::StartTag(STag { span, .. }) => Err(SvgParseError::new_span(
                        span,
                        SvgParseErrorMessage::MultipleRootElements,
                    ))?,
                    Element::EmptyTag(EmptyElemTag { span, .. })
                    | Element::EndTag(ETag { span, .. }) => Err(SvgParseError::new_span(
                        span,
                        SvgParseErrorMessage::UnexpectedTagType(TagType::Any),
                    ))?,
                    e => misc.push(Markup::Element(e)),
                },
                m => misc.push(m),
            };
        }
    }

    pub fn parse(svg: &str) -> Result<Self, Box<SvgParseError>> {
        let mut chars = svg.chars().peekable();
        Ok(Self::new(&mut chars, Cursor::default())?.1)
    }
}

#[derive(PartialEq, Debug)]
pub enum Node {
    EmptyNode(EmptyElemTag),
    ContentNode((Rc<RefCell<STag>>, Vec<NodeContent>, ETag)),
}

pub fn node(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    start_tag: Rc<RefCell<STag>>,
) -> Result<(Cursor, Rc<RefCell<Node>>), Box<SvgParseError>> {
    let node = Rc::new(RefCell::new(Node::ContentNode((
        Rc::clone(&start_tag),
        Vec::new(),
        ETag::default(),
    ))));
    let (cursor, content, end_tag) = content(partial, cursor, Rc::clone(&node))?;
    match &mut *node.borrow_mut() {
        Node::ContentNode((_, ref mut c, ref mut e)) => {
            *c = content;
            e.start_tag = start_tag;
            *e = end_tag;
        }
        _ => unreachable!(),
    }
    Ok((cursor, node))
}

#[test]
fn test_document() {
    dbg!(Document::parse("<svg attr=\"hi\">\n</svg>"));
}
