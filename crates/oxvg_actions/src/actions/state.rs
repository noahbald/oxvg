use oxvg_collections::{
    attribute::{
        AttrId,
        core_attrs::Integer,
        list_of::{ListOf, SpaceOrComma},
    },
    name::{Prefix, QualName},
};
use oxvg_parse::Parse as _;

use crate::{Action, Actor, Error, OXVG_PREFIX, effects::StateEffect, utils::to_id};

impl<'input> Actor<'input, '_> {
    /// Removes OXVG state from the document
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/forget.md")]
    pub fn forget(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Forget);

        self.effect_selection(&vec![].into())?;
        self.effect_state(StateEffect::Remove)?;
        if let Some(root) = self.root.find_element() {
            root.remove_attribute(&AttrId::Unknown(QualName {
                prefix: Prefix::XMLNS,
                local: OXVG_PREFIX.into(),
            }));
        }
        Ok(())
    }

    /// Updates the state of the actor to point to the elements matching the given selector.
    /// Elements can also be selected by a space/comma separated list of allocation-id
    /// integers.
    ///
    /// # Errors
    ///
    /// When root element is missing or the query cannot be parsed.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/select.md")]
    pub fn select(&mut self, query: &str) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Select(query.to_string().into()));
        self.state.state.remove();

        let selections = self.select_internal(query)?;
        self.effect_selection(&selections)
    }

    /// Updates the state of the actor to point to the elements matching the given selector,
    /// including any previous selections.
    /// Elements can also be selected by a space/comma separated list of allocation-id
    /// integers.
    ///
    /// # Errors
    ///
    /// When root element is missing or the query cannot be parsed.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/select-more.md")]
    pub fn select_more(&mut self, query: &str) -> Result<(), Error<'input>> {
        self.effect_history(&Action::SelectMore(query.to_string().into()));
        self.state.state.remove();

        let mut selections = self.get_selections_list()?.unwrap_or_default();
        let new_selections = self.select_internal(query)?;
        selections.list.extend(new_selections.list);

        self.effect_selection(&selections)
    }

    /// Selects the first-child of the current selection. Does nothing if the selection has no children.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/first_child.md")]
    pub fn first_child(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::FirstChild);
        self.state.state.remove();

        let selections = self.get_selections()?;
        let new_selections = self
            .get_selection_nodes(selections)
            .map(|node| node.first_child().unwrap_or(node))
            .map(to_id);

        self.effect_selection(&new_selections.collect())
    }

    /// Selects the previous-sibling of the current selection. Does nothing if the selection is the first-child.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/previous_sibling.md")]
    pub fn previous_sibling(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::PreviousSibling);
        self.state.state.remove();

        let selections = self.get_selections()?;
        let new_selections = self
            .get_selection_nodes(selections)
            .map(|node| node.previous_sibling().unwrap_or(node))
            .map(to_id);

        self.effect_selection(&new_selections.collect())
    }

    /// Selects the next-sibling of the current selection. Does nothing if the selection is the first-child.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/next_sibling.md")]
    pub fn next_sibling(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::NextSibling);
        self.state.state.remove();

        let selections = self.get_selections()?;
        let new_selections = self
            .get_selection_nodes(selections)
            .map(|node| node.next_sibling().unwrap_or(node))
            .map(to_id);

        self.effect_selection(&new_selections.collect())
    }

    /// Selects the last-child of the current selection. Does nothing if the selection has no children.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/last_child.md")]
    pub fn last_child(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::LastChild);
        self.state.state.remove();

        let selections = self.get_selections()?;
        let new_selections = self
            .get_selection_nodes(selections)
            .map(|node| node.last_child().unwrap_or(node))
            .map(to_id);

        self.effect_selection(&new_selections.collect())
    }

    /// Updates the state of the actor to deselected any selected nodes.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/deselect.md")]
    pub fn deselect(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Deselect);
        self.effect_selection(&vec![].into())
    }

    fn select_internal(
        &mut self,
        query: &str,
    ) -> Result<ListOf<Integer, SpaceOrComma>, Error<'static>> {
        if query.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            ListOf::parse_string(query).map_err(|err| Error::ParseError(err.to_string()))
        } else {
            let Some(root) = self.root.element() else {
                return Err(Error::NoRootElement);
            };
            let elements = root
                .select(query)
                .map_err(|_| Error::InvalidSelector(query.to_string()))?;

            Ok(elements.map(|e| to_id(*e)).collect())
        }
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn select_empty() {
        oxvg_ast::parse::roxmltree::parse(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100"/>"#,
        |root, allocator| {
            let mut actor = Actor::new(root, allocator).unwrap();

            actor.select("svg").unwrap();
            insta::assert_snapshot!(actor.root.serialize().unwrap());

            actor.select("1").unwrap();
            insta::assert_snapshot!(actor.root.serialize().unwrap());
        },
    )
    .unwrap();
    }

    #[test]
    fn select() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <g color="black"/>
    <g color="BLACK"/>
    <path fill="rgb(64 64 64)"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());

                actor.select("7, 9").unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn first_child() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.first_child().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn previous_sibling() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("two").unwrap();
                actor.previous_sibling().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn next_sibling() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("one").unwrap();
                actor.next_sibling().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn last_child() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><one/><two/></svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.last_child().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn deselect() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <g color="black"/>
    <g color="BLACK"/>
    <path fill="rgb(64 64 64)"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                actor.deselect().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn forget() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100"/>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                actor.forget().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
