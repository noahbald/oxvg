#![feature(mapped_lock_guards)]
/*!
Similar to Inkscape's actions, can be used to manipulate a document through a CLI.

As a library, actions can be used to dispatch changes to a document from an editor.
*/
mod actions;
mod error;
pub(crate) mod info;
pub(crate) mod state;
mod utils;

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

pub use actions::{Action, Actor};
pub use error::Error;
pub use info::{AttrModel, ContentModel, Info};
pub use state::DerivedState;

#[cfg(feature = "napi")]
pub use actions::ActionNapi;
#[cfg(feature = "napi")]
pub use state::DerivedStateNapi;

/// The `xmlns:oxvg` value for OXVG elements.
pub const OXVG_XMLNS: &str = "https://oxvg.noahwbaldwin.me";
/// The prefix assigned via `xmlns:oxvg`.
pub const OXVG_PREFIX: &str = "oxvg";
