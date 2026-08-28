use oxvg_ast::{arena::Allocator, node::Ref, serialize::Node};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core_attrs::{Integer, Number},
        list_of::{ListOf, SpaceOrComma},
    },
};

use oxvg_parse::Parse as _;
#[cfg(feature = "wasm")]
use tsify::Tsify;

mod manipulate;
mod state;
mod structure;
mod transform;

use crate::{
    effects::StateEffect,
    error::Error,
    state::{DerivedState, State, StateElement},
    utils::get_oxvg_attr,
};

/// An actor holds a reference to a document to act upon.
///
/// The actor will embed it's state into the document upon parsing and serializing.
pub struct Actor<'input, 'arena> {
    /// The root of the document for the actor to act upon
    pub root: Ref<'input, 'arena>,
    /// The allocator associated with the given document
    pub allocator: Allocator<'input, 'arena>,
    pub(crate) state: State<'input, 'arena>,
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
    /// See [`Actor::path_intersect`]
    PathIntersect,
    /// See [`Actor::path_union`]
    PathUnion,
    /// See [`Actor::path_subtract`]
    PathSubtract,
    /// See [`Actor::path_xor`]
    PathXor,
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
    /// See [`Actor::insert`]
    Insert(Atom<'input>),
    /// See [`Actor::insert_ns`]
    InsertNS(Atom<'input>, Atom<'input>),
    /// See [`Actor::duplicate`]
    Duplicate,
    /// See [`Actor::wrap`]
    Wrap(Atom<'input>),
    /// See [`Actor::clone`]
    Clone,
    /// See [`Actor::anchor_link`]
    AnchorLink(Atom<'input>),
    /// See [`Actor::group`]
    Group,
    /// See [`Actor::delete`]
    Delete,
    /// See [`Actor::flatten`]
    Flatten,
    /// See [`Actor::front`]
    Front,
    /// See [`Actor::push`]
    Push,
    /// See [`Actor::pull`]
    Pull,
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
    /// See [`Actor::path_intersect`]
    PathIntersect,
    /// See [`Actor::path_union`]
    PathUnion,
    /// See [`Actor::path_subtract`]
    PathSubtract,
    /// See [`Actor::path_xor`]
    PathXor,
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
    /// See [`Actor::insert`]
    Insert(String),
    /// See [`Actor::insert_ns`]
    InsertNS(String, String),
    /// See [`Actor::duplicate`]
    Duplicate,
    /// See [`Actor::wrap`]
    Wrap(String),
    /// See [`Actor::clone`]
    Clone,
    /// See [`Actor::anchor_link`]
    AnchorLink(String),
    /// See [`Actor::group`]
    Group,
    /// See [`Actor::delete`]
    Delete,
    /// See [`Actor::flatten`]
    Flatten,
    /// See [`Actor::front`]
    Front,
    /// See [`Actor::push`]
    Push,
    /// See [`Actor::pull`]
    Pull,
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
        self.effect_state(StateEffect::Embed)?;
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
            Action::Attr { name, value } => self.attr(&name, &value),
            Action::Class(name) => self.class(&name),
            Action::Style { property, value } => self.style(&property, &value),
            Action::PathIntersect => self.path_intersect(),
            Action::PathUnion => self.path_union(),
            Action::PathSubtract => self.path_subtract(),
            Action::PathXor => self.path_xor(),
            Action::Matrix(a, b, c, d, e, f) => self.matrix(a, b, c, d, e, f),
            Action::Translate(x, y) => self.translate(x, y),
            Action::Scale(x, y) => self.scale(x, y),
            Action::Rotate(angle, origin) => self.rotate(angle, origin),
            Action::SkewX(angle) => self.skew_x(angle),
            Action::SkewY(angle) => self.skew_y(angle),
            Action::Insert(name) => self.insert(&name),
            Action::InsertNS(uri, name) => self.insert_ns(&uri, &name),
            Action::Duplicate => self.duplicate(),
            Action::Wrap(name) => self.wrap(&name),
            Action::Clone => self.clone(),
            Action::AnchorLink(href) => self.anchor_link(&href),
            Action::Group => self.group(),
            Action::Delete => self.delete(),
            Action::Flatten => self.flatten(),
            Action::Front => self.front(),
            Action::Push => self.push(),
            Action::Pull => self.pull(),
            Action::Forget => self.forget(),
            Action::Select(query) => self.select(&query),
            Action::SelectMore(query) => self.select_more(&query),
            Action::Deselect => self.deselect(),
        }
    }

    pub(crate) fn get_selections_list(
        &mut self,
    ) -> Result<Option<ListOf<Integer, SpaceOrComma>>, Error<'input>> {
        let selections_element = self.state.get_selections(&self.allocator);
        let Some(selections) = get_oxvg_attr(&selections_element, StateElement::SELECTION_IDS)?
        else {
            return Ok(None);
        };
        ListOf::<Integer, SpaceOrComma>::parse_string(&selections)
            .map_err(|err| Error::ParseError(err.to_string()))
            .map(Some)
    }

    pub(crate) fn get_selections(&mut self) -> Result<Option<Vec<Integer>>, Error<'input>> {
        Ok(self.get_selections_list()?.map(|s| s.list))
    }
}
