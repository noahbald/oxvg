use oxvg_ast::{arena::Allocator, node::Ref, serialize::Node};
use oxvg_collections::{atom::Atom, attribute::core_attrs::Number};

#[cfg(feature = "wasm")]
use tsify::Tsify;

mod manipulate;
mod state;
mod transform;

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
    /// See [`Actor::attr`]
    Attr {
        /// The qualified name of the attribute
        name: Atom<'input>,
        /// The value of the attribute
        value: Atom<'input>,
    },
    /// See [`Actor::class`]
    Class(Atom<'input>),
    /// See [`Actor::style`]
    Style {
        /// The CSS name of the property
        property: Atom<'input>,
        /// The CSS value of the property
        value: Atom<'input>,
    },
    /// See [`Actor::matrix`]
    Matrix(Number, Number, Number, Number, Number, Number),
    /// See [`Actor::translate`]
    Translate(Number, Option<Number>),
    /// See [`Actor::scale`]
    Scale(Number, Option<Number>),
    /// See [`Actor::rotate`]
    Rotate(Number, Option<(Number, Number)>),
    /// See [`Actor::skew_x`]
    SkewX(Number),
    /// See [`Actor::skew_y`]
    SkewY(Number),
    /// See [`Actor::forget`]
    Forget,
    /// See [`Actor::select`]
    Select(Atom<'input>),
    /// See [`Actor::select_more`]
    SelectMore(Atom<'input>),
    /// See [`Actor::deselect`]
    Deselect,
}

#[cfg(feature = "napi")]
#[napi]
/// An action is a method that an actor can execute upon a document
pub enum ActionNapi {
    /// See [`Actor::attr`]
    Attr {
        /// The qualified name of the attribute
        name: String,
        /// The value of the attribute
        value: String,
    },
    /// See [`Actor::class`]
    Class(String),
    /// See [`Actor::style`]
    Style {
        /// The CSS name of the property
        property: String,
        /// The CSS value of the property
        value: String,
    },
    /// See [`Actor::matrix`]
    Matrix(f64, f64, f64, f64, f64, f64),
    /// See [`Actor::translate`]
    Translate(f64, Option<f64>),
    /// See [`Actor::scale`]
    Scale(f64, Option<f64>),
    /// See [`Actor::rotate`]
    Rotate(f64, Option<(f64, f64)>),
    /// See [`Actor::skew_x`]
    SkewX(f64),
    /// See [`Actor::skew_y`]
    SkewY(f64),
    /// See [`Actor::forget`]
    Forget,
    /// See [`Actor::select`]
    Select(String),
    /// See [`Actor::select_more`]
    SelectMore(String),
    /// See [`Actor::deselect`]
    Deselect,
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

    #[allow(clippy::many_single_char_names)]
    /// Executes the given action and it's arguments upon the document.
    ///
    /// # Errors
    ///
    /// When the associated action fails
    pub fn dispatch(&mut self, action: Action<'input>) -> Result<(), Error<'input>> {
        match action {
            Action::Attr { name, value } => return self.attr(&name, &value),
            Action::Class(name) => return self.class(&name),
            Action::Style { property, value } => return self.style(&property, &value),
            Action::Matrix(a, b, c, d, e, f) => return self.matrix(a, b, c, d, e, f),
            Action::Translate(x, y) => return self.translate(x, y),
            Action::Scale(x, y) => return self.scale(x, y),
            Action::Rotate(angle, origin) => return self.rotate(angle, origin),
            Action::SkewX(angle) => return self.skew_x(angle),
            Action::SkewY(angle) => return self.skew_y(angle),
            Action::Forget => self.forget(),
            Action::Select(query) => return self.select(&query),
            Action::SelectMore(query) => return self.select_more(&query),
            Action::Deselect => self.deselect(),
        }
        Ok(())
    }
}
