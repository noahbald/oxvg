use oxvg_collections::attribute::{
    core_attrs::Integer,
    list_of::{ListOf, SpaceOrComma},
};
use oxvg_parse::Parse as _;
use oxvg_serialize::{PrinterOptions, ToValue as _};

use crate::{
    state::StateElement,
    utils::{create_oxvg_attr, create_oxvg_attr_id},
    Action, Actor, Error,
};

impl<'input, 'arena> Actor<'input, 'arena> {
    /// Removes OXVG state from the document
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/state/forget.md")]
    pub fn forget(&mut self) {
        self.state.record(&Action::Forget, &self.allocator);

        self.state
            .get_selections(&self.allocator)
            .remove_attribute(&create_oxvg_attr_id(StateElement::SELECTION_IDS));
        self.state.state.remove();
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
        let Some(root) = self.root.element() else {
            return Err(Error::NoRootElement);
        };
        self.state
            .record(&Action::Select(query.to_string().into()), &self.allocator);

        let selections: ListOf<Integer, SpaceOrComma> =
            if query.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                ListOf::parse_string(query).map_err(|err| Error::ParseError(err.to_string()))?
            } else {
                let elements = root
                    .select(query)
                    .map_err(|_| Error::InvalidSelector(query.to_string()))?;

                #[allow(clippy::cast_possible_wrap)]
                let selections: Vec<_> = elements.map(|e| e.id() as Integer).collect();

                ListOf {
                    list: selections,
                    separator: SpaceOrComma,
                }
            };
        self.state
            .get_selections(&self.allocator)
            .set_attribute(create_oxvg_attr(
                StateElement::SELECTION_IDS,
                selections
                    .to_value_string(PrinterOptions::default())
                    .map_err(|err| Error::SerializeError(err.to_string()))?
                    .into(),
            ));
        self.state.embed(self.root)?;
        Ok(())
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
}
