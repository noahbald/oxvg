pub mod attributes;

use oxvg_diagnostics::SVGError;
use rcdom::Node;

pub trait Rule {
    fn execute(&self, element: &Node) -> Vec<SVGError>;
}
