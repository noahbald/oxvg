//! The arena used to allocate nodes
use std::cell::Cell;

use crate::node::{Node, NodeData};

/// The inner value of [`Arena`]
type PrivateArena<'input, 'arena> = &'arena typed_arena::Arena<Node<'input, 'arena>>;
/// An arena for [`Node`] values
pub struct Arena<'input, 'arena>(typed_arena::Arena<Node<'input, 'arena>>);

/// The inner value of [`Values`]
type PrivateValues<'input> = &'input typed_arena::Arena<u8>;
/// An arena for `&'input str` values
///
/// Used for copying values into a document which may not live for `'input`.
///
/// For inputs that do live for `'input`, try using [`Atom::from`]
pub struct Values(typed_arena::Arena<u8>);

/// The allocator for adding new data points that live as long as the document
#[derive(Clone)]
pub struct Allocator<'input, 'arena> {
    /// The arena for new nodes
    arena: PrivateArena<'input, 'arena>,
    /// The arena for new strings
    values: PrivateValues<'input>,
    /// Incrementally counts the number of allocated nodes to assign as the id of allocated nodes
    current_node_id: Cell<usize>,
}
impl<'input, 'arena> Allocator<'input, 'arena> {
    /// Returns an arena that cannot be publicly accessed
    pub fn new_arena() -> Arena<'input, 'arena> {
        Arena(typed_arena::Arena::new())
    }

    /// Returns an arena that cannot be publicly accessed
    pub fn new_arena_with_capacity(n: usize) -> Arena<'input, 'arena> {
        Arena(typed_arena::Arena::with_capacity(n))
    }

    /// Returns a value arena that cannot be publicly accessed
    pub fn new_values() -> Values {
        Values(typed_arena::Arena::new())
    }

    /// Creates a new allocator to assign nodes and strings that live as long as the document requires.
    ///
    /// The allocator allows you to allocate
    pub fn new(
        // NOTE: Arena is `mut` to prevent sharing, otherwise the invariant of a unique `current_node_id`
        // may be broken.
        // Users should use `Arena` sequentially rather than at the same time.
        arena: &'arena mut Arena<'input, 'arena>,
        values: &'input Values,
    ) -> Self {
        Self {
            arena: &arena.0,
            values: &values.0,
            current_node_id: Cell::new(arena.0.len()),
        }
    }

    /// Allocates a node with the given [`NodeData`]
    pub fn alloc(&self, node_data: NodeData<'input>) -> &'arena mut Node<'input, 'arena> {
        let id = self.current_node_id.get();
        self.current_node_id.set(id + 1);
        self.arena
            .alloc(Node::new(node_data, self.current_node_id.get()))
    }

    /// Allocates a string to live as long as `'input`
    ///
    /// # Performance
    ///
    /// This method copies the string for the given lifetime. You may prefer to use [`crate::atom::Atom`]
    /// when possible.
    pub fn alloc_str(&self, str: &str) -> &'input mut str {
        self.values.alloc_str(str)
    }
}
