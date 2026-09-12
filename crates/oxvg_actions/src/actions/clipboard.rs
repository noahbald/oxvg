use crate::{Action, Actor, Error, actions::UIAction, utils::to_id};

impl<'input> Actor<'input, '_> {
    /// Copy the selected element(s) to clipboard, separated by newline. Adds deep
    /// copy of selected elements to <oxvg:clipboard>.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/ui/copy.md")]
    pub fn copy(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Copy);

        let selections = self.get_selections()?;

        self.state.state.remove();
        self.effect_clipboard(self.get_selection_nodes(selections).collect())?;
        self.effect_ui(UIAction::Copy)
    }

    /// Uses the content in `<oxvg:clipboard>`. If empty, does nothing. Selects the root of the
    /// pasted tree.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/ui/paste.md")]
    pub fn paste(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Paste);

        let clipboard = self.state.get_clipboard(&self.allocator);
        let selections = self.get_selections()?;
        let mut new_selections = Vec::with_capacity(
            selections.as_ref().map_or(0, Vec::len) * clipboard.child_node_count(),
        );

        for node in self.get_selection_nodes(selections) {
            for copy in clipboard.child_nodes_iter() {
                let copy = copy.clone_node(&self.allocator, true);
                new_selections.push(copy);
                node.append_child(copy);
            }
        }

        self.effect_selection(&new_selections.into_iter().map(to_id).collect())?;
        self.effect_tree()?;
        self.effect_document()
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn copy() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.copy().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn paste() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><g/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("g").unwrap();
                actor.copy().unwrap();
                actor.paste().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
