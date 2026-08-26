use crate::{Action, Actor, Error};

impl<'input> Actor<'input, '_> {
    /// Moves the selected elements to be in front of all it's siblings.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/front.md")]
    pub fn front(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Front);

        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        #[allow(clippy::cast_sign_loss)]
        for selection in selections
            .into_iter()
            .rev()
            .filter_map(|e| self.allocator.get(e as usize))
        {
            let Some(parent) = selection.parent_node() else {
                continue;
            };
            selection.remove();
            parent.prepend(selection);
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
    fn front() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("two").unwrap();
                actor.front().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn front_many() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two a=""/><three a=""/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("[a]").unwrap();
                actor.front().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
