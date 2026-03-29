use oxvg_ast::{arena::Allocator, node::Ref, serialize::Node};
use oxvg_collections::atom::Atom;

#[cfg(feature = "wasm")]
use tsify::Tsify;

mod state;

use crate::{
    error::Error,
    state::{DerivedState, State},
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
    /// See [`Actor::forget`]
    Forget,
    /// See [`Actor::select`]
    Select(Atom<'input>),
}

#[cfg(feature = "napi")]
#[napi]
/// An action is a method that an actor can execute upon a document
pub enum ActionNapi {
    /// See [`Actor::forget`]
    Forget,
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
            Action::Forget => self.forget(),
            Action::Select(query) => return self.select(query.as_str()),
        }
        Ok(())
    }
}
