use crate::{Action, Actor, Error, utils::to_id};

impl<'input> Actor<'input, '_> {
    /// Removes each selected element from the document. Deselects.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/delete.md")]
    pub fn delete(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Delete);

        let selections = self.get_selections()?;
        for selection in self.get_selection_nodes(selections) {
            selection.remove();
        }

        self.effect_selection(&vec![].into())?;
        self.effect_tree()?;
        self.effect_document()
    }

    /// Removes the selected elements, replacing itself with it's children. Selection moved to children.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/flatten.md")]
    pub fn flatten(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Flatten);

        let selections = self.get_selections()?;
        let mut new_selections = vec![];
        for selection in self.get_selection_elements(selections) {
            new_selections.extend(selection.child_nodes_iter().map(to_id));
            selection.flatten();
        }

        self.effect_selection(&new_selections.into())?;
        self.effect_tree()?;
        self.effect_document()
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn delete() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><path/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                actor.delete().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn flatten() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <g>
        <text>One</text>
        <text>Two</text>
    </g>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("g").unwrap();
                actor.flatten().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
