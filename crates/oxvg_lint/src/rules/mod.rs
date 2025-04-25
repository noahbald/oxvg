mod attributes;

use rcdom::Node;

/// A rule analyses a document and produces an error for any issues it finds.
pub trait Rule {
    /// Runs the rule against a document to assess for errors.
    fn execute(&self, element: &Node) -> Vec<String>;
}
