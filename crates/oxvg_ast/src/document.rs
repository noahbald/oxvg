use oxvg_diagnostics::SVGError;
use std::{cell::RefCell, rc::Rc};

use crate::node::{Element, Root};

// [2.1 Well-Formed XML Documents](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-well-formed)

#[derive(Debug)]
pub struct Document {
    /// The root of the document, containing all of it's parsed contents
    pub root: Rc<RefCell<Root>>,
    /// The first tag of the root
    pub root_element: Option<Rc<RefCell<Element>>>,
    /// A list of any errors encountered while parsing the document
    pub errors: Vec<SVGError>,
}
