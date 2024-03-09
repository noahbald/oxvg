use std::{cell::RefCell, rc::Rc};

// [2.1 Well-Formed XML Documents](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-well-formed)
use crate::{
    content,
    diagnostics::SVGError,
    file_reader::{Element, FileReader, Root},
    ETag, EmptyElemTag, NodeContent, STag,
};

#[derive(Debug)]
pub struct Document {
    pub root: Root,
    pub root_element: Option<Rc<RefCell<Element>>>,
    pub errors: Vec<SVGError>,
}

impl Document {
    pub fn parse(svg: &str) -> Self {
        let mut file_reader = FileReader::new(svg);
        file_reader.collect_root();
        file_reader.into()
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
) -> Result<Rc<RefCell<Node>>, Box<SVGError>> {
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
