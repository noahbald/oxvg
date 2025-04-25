/*!
Similar to Inkscape's actions, can be used to manipulate a document through a CLI.

As a library, actions can be used to dispatch changes to a document from an editor.
*/
mod actions;
pub mod base;
pub mod tutorial;

pub use actions::Actions;
