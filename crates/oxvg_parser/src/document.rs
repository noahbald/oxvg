use std::{cell::RefCell, rc::Rc};

// [2.1 Well-Formed XML Documents](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-well-formed)
use crate::{
    diagnostics::SVGError,
    file_reader::{Element, FileReader, Root},
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

#[test]
fn test_document() {
    dbg!(Document::parse("<svg attr=\"hi\">\n</svg>"));
}
