use crate::node::Node;

pub type Arena<'arena> = &'arena typed_arena::Arena<Node<'arena>>;
