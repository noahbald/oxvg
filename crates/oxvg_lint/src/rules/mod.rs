pub mod attributes;

use oxvg_ast::Child;
use oxvg_diagnostics::SVGError;

pub trait Rule {
    fn execute(&self, element: Child) -> Vec<SVGError>;
}
