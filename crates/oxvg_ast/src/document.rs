use oxvg_diagnostics::SVGError;

// [2.1 Well-Formed XML Documents](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-well-formed)

#[derive(Debug)]
pub struct Document {
    /// A list of any errors encountered while parsing the document
    pub errors: Vec<SVGError>,
}
