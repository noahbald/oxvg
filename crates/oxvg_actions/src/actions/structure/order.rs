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

        let selections = self.get_selections()?;
        for selection in self.get_selection_nodes(selections).rev() {
            let Some(parent) = selection.parent_node() else {
                continue;
            };
            selection.remove();
            parent.prepend(selection);
        }

        self.effect_tree()?;
        self.effect_document()
    }

    /// Moves the selected elements to be in front of their next sibling.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/push.md")]
    pub fn push(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Push);

        let selections = self.get_selections()?;
        for selection in self.get_selection_nodes(selections).rev() {
            let Some(next_sibling) = selection.next_sibling() else {
                continue;
            };
            let Some(parent) = selection.parent_node() else {
                continue;
            };
            parent.insert_after(selection, next_sibling);
        }

        self.effect_tree()?;
        self.effect_document()
    }

    /// Moves the selected elements to be behind their previous sibling.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/pull.md")]
    pub fn pull(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Pull);

        let selections = self.get_selections()?;
        for selection in self.get_selection_nodes(selections) {
            let Some(previous_sibling) = selection.previous_sibling() else {
                continue;
            };
            let Some(parent) = selection.parent_node() else {
                continue;
            };
            parent.insert_before(selection, previous_sibling);
        }

        self.effect_tree()?;
        self.effect_document()
    }

    /// Moves the selected elements to be behind all of it's siblings.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/structure/back.md")]
    pub fn back(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Back);

        let selections = self.get_selections()?;
        for selection in self.get_selection_nodes(selections) {
            let Some(parent) = selection.parent_node() else {
                continue;
            };
            selection.remove();
            parent.append(selection);
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

    #[test]
    fn push() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/><three/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("two").unwrap();
                actor.push().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn push_many_2() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/><three/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("one,two").unwrap();
                actor.push().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn push_many_3() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/><three/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("one,two,three").unwrap();
                actor.push().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn pull() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/><three/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("two").unwrap();
                actor.pull().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
