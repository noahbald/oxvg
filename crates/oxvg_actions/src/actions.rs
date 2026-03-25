use oxvg_ast::{
    arena::Allocator,
    node::Ref,
    serialize::{Node, PrinterOptions, ToValue},
};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core_attrs::Integer,
        list_of::{ListOf, SpaceOrComma},
    },
};

use oxvg_parse::Parse;
#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::{
    error::Error,
    state::{DerivedState, State, StateElement},
    utils::create_oxvg_attr,
};

/// An actor holds a reference to a document to act upon.
///
/// The actor will embed it's state into the document upon parsing and serializing.
pub struct Actor<'input, 'arena> {
    /// The root of the document for the actor to act upon
    pub root: Ref<'input, 'arena>,
    /// The allocator associated with the given document
    pub allocator: Allocator<'input, 'arena>,
    state: State<'input, 'arena>,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(from_wasm_abi, into_wasm_abi))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// An action is a method that an actor can execute upon a document
pub enum Action<'input> {
    /// See [`Actor::select`]
    Select(Atom<'input>),
}

#[cfg(feature = "napi")]
#[napi]
/// An action is a method that an actor can execute upon a document
pub enum ActionNapi {
    /// See [`Actor::select`]
    Select(String),
}

impl<'input, 'arena> Actor<'input, 'arena> {
    /// Creates a new actor with a reference to the document. The state of the actor will be
    /// derived from the document's `oxvg:state` element.
    ///
    /// # Errors
    ///
    /// If state element is invalid
    pub fn new(
        root: Ref<'input, 'arena>,
        allocator: Allocator<'input, 'arena>,
    ) -> Result<Self, Error<'input>> {
        Ok(Actor {
            root,
            state: State::debed(root, &allocator)?,
            allocator,
        })
    }

    /// Returns a serialized document containing the updated document with any embedded state.
    ///
    /// # Errors
    ///
    /// If serialization fails, or if the document is missing a root element.
    pub fn snapshot(&mut self) -> Result<String, Error<'input>> {
        self.state.embed(self.root)?;
        self.root
            .serialize()
            .map_err(|err| Error::SerializeError(err.to_string()))
    }

    /// Returns a rich state object based on the `oxvg:state` embedded in the document
    ///
    /// # Errors
    ///
    /// When any invalid state element data is encountered
    pub fn derive_state(&self) -> Result<DerivedState<'input>, Error<'input>> {
        DerivedState::from_state(&self.state, &self.allocator)
    }

    /// Executes the given action and it's arguments upon the document.
    ///
    /// # Errors
    ///
    /// When the associated action fails
    pub fn dispatch(&mut self, action: Action<'input>) -> Result<(), Error<'input>> {
        match action {
            Action::Select(query) => self.select(query.as_str()),
        }
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
    #[doc = include_str!("./spec/select.md")]
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
