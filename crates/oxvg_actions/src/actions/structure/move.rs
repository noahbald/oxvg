use oxvg_ast::node::Node;

use crate::{Action, Actor, Error};

impl<'input> Actor<'input, '_> {
    /// Moves the selected element into the start of it's next sibling. Does nothing if it's the last child.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/step_in.md")]
    pub fn step_in(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::StepIn);

        let selections = self.get_selections()?;
        for node in self.get_selection_nodes(selections) {
            let Some(next_sibling) = node.next_sibling().and_then(Node::element) else {
                continue;
            };
            node.remove();
            next_sibling.prepend(node);
        }

        self.effect_tree()?;
        self.effect_document()
    }

    /// Moves the selected element up to be behind it's parent.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/step_out.md")]
    pub fn step_out(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::StepOut);

        let selections = self.get_selections()?;
        for node in self.get_selection_nodes(selections) {
            let Some(parent) = node.parent_node() else {
                continue;
            };
            let Some(grand_parent) = parent.parent_node() else {
                continue;
            };
            node.remove();
            grand_parent.insert_before(node, *parent);
        }

        self.effect_tree()?;
        self.effect_document()
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn step_in() {
        oxvg_ast::parse::roxmltree::parse(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100"><base/><outer><one/><two/></outer></svg>"#,
        |root, allocator| {
            let mut actor = Actor::new(root, allocator).unwrap();

            actor.select("base").unwrap();
            actor.step_in().unwrap();
            insta::assert_snapshot!(actor.root.serialize().unwrap());
            insta::assert_debug_snapshot!(actor.derive_state().unwrap());
        },
    )
    .unwrap();
    }

    #[test]
    fn step_out() {
        oxvg_ast::parse::roxmltree::parse(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100"><outer><one/><two/></outer></svg>"#,
        |root, allocator| {
            let mut actor = Actor::new(root, allocator).unwrap();

            actor.select("two").unwrap();
            actor.step_out().unwrap();
            insta::assert_snapshot!(actor.root.serialize().unwrap());
            insta::assert_debug_snapshot!(actor.derive_state().unwrap());
        },
    )
    .unwrap();
    }
}
