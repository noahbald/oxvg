// [2.1 Well-Formed XML Documents](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-well-formed)
use crate::{
    content, cursor::Cursor, diagnostics::SvgParseError, file_reader::FileReader, markup, ETag,
    Element, EmptyElemTag, Markup, NodeContent, STag, SvgParseErrorMessage, TagType,
};
use std::{cell::RefCell, iter::Peekable, rc::Rc};

#[derive(Debug)]
pub struct Document {
    pub prolog: Vec<Markup>,
    pub element: Rc<RefCell<Node>>,
    pub misc: Vec<Markup>,
}

impl Document {
    pub fn new(file_reader: &mut FileReader) -> Result<Self, Box<SvgParseError>> {
        loop {
            let collected_state = file_reader.collect_state();
            match collected_state {
                Some(state) => println!("{:?}: {}", state.state_collected, state.contents),
                None => Err(SvgParseError::new_curse(
                    file_reader.get_cursor(),
                    SvgParseErrorMessage::Generic("This is fine".into()),
                ))?,
            }
        }
        // [Document](https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-document)
        // [1]
        let mut prolog = Vec::new();
        let root_start = Rc::new(RefCell::new(STag::default()));
        loop {
            match markup(file_reader, None)? {
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
        let element = node(file_reader, root_start)?;

        let mut misc = Vec::new();
        loop {
            match markup(file_reader, None)? {
                Markup::Element(e) => match e {
                    Element::EndOfFile => {
                        return Ok(Document {
                            prolog,
                            element,
                            misc,
                        })
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
        let mut file_reader = FileReader::new(svg);
        Self::new(&mut file_reader)
    }
}

#[derive(Debug)]
pub enum Node {
    EmptyNode(EmptyElemTag),
    ContentNode((Rc<RefCell<STag>>, Vec<NodeContent>, ETag)),
}

pub fn node(
    file_reader: &mut FileReader,
    start_tag: Rc<RefCell<STag>>,
) -> Result<Rc<RefCell<Node>>, Box<SvgParseError>> {
    let node = Rc::new(RefCell::new(Node::ContentNode((
        Rc::clone(&start_tag),
        Vec::new(),
        ETag::default(),
    ))));
    let (content, end_tag) = content(file_reader, Rc::clone(&node))?;
    match &mut *node.borrow_mut() {
        Node::ContentNode((_, ref mut c, ref mut e)) => {
            *c = content;
            e.start_tag = start_tag;
            *e = end_tag;
        }
        _ => unreachable!(),
    }
    Ok(node)
}

#[test]
fn test_document() {
    dbg!(Document::parse("<svg attr=\"hi\">\n</svg>"));
}
