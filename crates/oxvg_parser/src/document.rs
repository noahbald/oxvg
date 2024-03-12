use std::{cell::RefCell, rc::Rc};

// [2.1 Well-Formed XML Documents](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-well-formed)
use crate::{
    diagnostics::SVGError,
    file_reader::{Element, FileReader, Root},
};

#[derive(Debug)]
pub struct Document {
    /// The root of the document, containing all of it's parsed contents
    pub root: Rc<RefCell<Root>>,
    /// The first tag of the root
    pub root_element: Option<Rc<RefCell<Element>>>,
    /// A list of any errors encountered while parsing the document
    pub errors: Vec<SVGError>,
}

impl Document {
    /// Parses the given string, returning a `Document` with the generated tree of elements
    ///
    /// # Example
    /// ```
    /// use oxvg_parser::Document;
    ///
    /// let document = Document::parse("<svg attr=\"hi\">\n</svg>");
    /// assert!(document.root_element.is_some());
    /// ```
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
