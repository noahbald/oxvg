pub mod atom;
pub mod attribute;
pub mod element;
pub mod implementations;
pub mod name;
pub mod node;

#[cfg(feature = "parse")]
pub mod parse;

#[cfg(feature = "serialize")]
pub mod serialize;

#[cfg(feature = "selectors")]
pub mod selectors;

pub trait ShallowClone {
    /// Clone the Rc without cloning the contained the node data
    fn as_owned(&self) -> Self;
}
