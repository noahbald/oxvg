pub mod attributes;

use oxvg_parser::{Child, SVGError};

pub trait Rule {
    fn execute(&self, element: Child) -> Vec<SVGError>;
}
