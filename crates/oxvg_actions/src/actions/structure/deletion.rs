use crate::{Action, Actor, Error};

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

        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        #[allow(clippy::cast_sign_loss)]
        for selection in selections
            .into_iter()
            .filter_map(|e| self.allocator.get(e as usize))
        {
            selection.remove();
        }

        self.effect_selection(&vec![].into())?;
        self.effect_tree()?;
        self.effect_document()
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn insert() {
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
}
