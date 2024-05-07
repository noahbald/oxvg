#[macro_use]
extern crate markup5ever;

mod document;
pub mod node;

pub use crate::document::Document;
pub use crate::node::Attributes;
