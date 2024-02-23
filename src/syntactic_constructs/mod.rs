mod literals;
mod names;
mod whitespace;

// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

pub use self::{
    literals::{literal, Literal, LiteralValue},
    names::Name,
    whitespace::whitespace,
};
