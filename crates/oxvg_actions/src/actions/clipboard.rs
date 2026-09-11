use crate::{Action, Actor, Error, actions::UIAction};

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
}
