mod characters;
mod names;
mod references;
mod whitespace;

// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

pub use self::{
    characters::{is_char, is_restricted_char},
    names::Name,
    references::{ENTITIES, XML_ENTITIES},
    whitespace::is_whitespace,
};
