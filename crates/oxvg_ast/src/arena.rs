//! The arena used to allocate nodes
use std::{cell::RefCell, marker::PhantomData};

use crate::node::{AllocationID, Node, NodeData, Ref};

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
/// For inputs that do live for `'input`, try using [`oxvg_collections::atom::Atom::from`]
pub struct Values(typed_arena::Arena<u8>);

/// The allocator for adding new data points that live as long as the document
#[derive(Clone)]
pub struct Allocator<'input, 'arena> {
    /// The arena for new nodes
    arena: PrivateArena<'input, 'arena>,
    /// The arena for new strings
    values: PrivateValues<'input>,
    /// Contains a mapping of each element to it's id
    indices: RefCell<Vec<usize>>,
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
        let mut indices = vec![];
        let len = arena.0.len();
        for node in arena.0.iter_mut() {
            if indices.is_empty() {
                indices = vec![std::ptr::from_ref(node) as usize; len];
            }
            indices[node.id()] = std::ptr::from_ref(node) as usize;
        }

        Self {
            arena: &arena.0,
            values: &values.0,
            indices: RefCell::new(indices),
        }
    }

    /// Allocates a node with the given [`NodeData`]
    pub fn alloc(&self, node_data: NodeData<'input>) -> &'arena mut Node<'input, 'arena> {
        let mut indices = self.indices.borrow_mut();
        let id = indices.len();
        let node = self.arena.alloc(Node::new(node_data, id));
        indices.push(std::ptr::from_ref(node) as usize);
        node
    }

    /// Allocates a string to live as long as `'input`
    ///
    /// # Performance
    ///
    /// This method copies the string for the given lifetime. You may prefer to use [`oxvg_collections::atom::Atom`]
    /// when possible.
    pub fn alloc_str(&self, str: &str) -> &'input mut str {
        self.values.alloc_str(str)
    }

    /// Returns the node associated with the given allocation id.
    ///
    /// # Panics
    ///
    /// If the allocator's id and the node's id become out of sync.
    pub fn get(&self, id: AllocationID) -> Option<Ref<'input, 'arena>> {
        self.indices.borrow().get(id).map(|&p| {
            let node = ptr_cast(p);

            assert!(node.id() == id);
            node
        })
    }

    /// Returns an iterator that returns all allocated nodes in order of allocation id
    pub fn iter(&self) -> Iter<'input, 'arena> {
        Iter {
            index: 0,
            indices: RefCell::clone(&self.indices),
            marker: PhantomData,
        }
    }

    /// Reorders allocations to match the ordering of the given root and it's
    /// descendants.
    /// Returns the length of the tree.
    ///
    /// Nodes outside of the tree's set have preserved order.
    pub fn reorder(&self, root: Ref<'input, 'arena>) -> usize {
        // Move all ids to out of range
        let len = self.arena.len();
        for node in self {
            *node.id.write().unwrap() = len;
        }
        // Assign ids in order to tree
        let tree_len = Self::reorder_internal(root, 0);
        // Reassign ids in order outside of tree
        let mut index = tree_len;
        for node in self {
            if *node.id.read().unwrap() == len {
                *node.id.write().unwrap() = index;
                index += 1;
            }
        }
        // Sort by id
        self.indices.borrow_mut().sort_by(|&a, &b| {
            let a = ptr_cast(a);
            let b = ptr_cast(b);
            a.id.read().unwrap().cmp(&b.id.read().unwrap())
        });
        tree_len
    }

    fn reorder_internal(node: Ref<'input, 'arena>, mut id: usize) -> usize {
        debug_assert!(
            *node.id.read().unwrap() <= id,
            "cannot reorder tree with cycles"
        );
        *node.id.write().unwrap() = id;
        id += 1;
        for node in node.child_nodes_iter() {
            id = Self::reorder_internal(node, id);
        }
        id
    }
}

impl<'input, 'arena> IntoIterator for &Allocator<'input, 'arena> {
    type Item = Ref<'input, 'arena>;
    type IntoIter = Iter<'input, 'arena>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            index: 0,
            indices: RefCell::clone(&self.indices),
            marker: PhantomData,
        }
    }
}

impl<'input, 'arena> rayon::iter::IntoParallelIterator for &Allocator<'input, 'arena> {
    type Item = Ref<'input, 'arena>;
    type Iter = rayon::iter::Map<rayon::vec::IntoIter<usize>, fn(usize) -> Self::Item>;

    fn into_par_iter(self) -> Self::Iter {
        use rayon::iter::ParallelIterator as _;
        self.indices.borrow().clone().into_par_iter().map(ptr_cast)
    }
}

fn ptr_cast<'input, 'arena>(p: usize) -> Ref<'input, 'arena> {
    unsafe { &*(p as *const Node<'input, 'arena>) }
}

/// An iterator that returns all allocated nodes in order of allocation id
pub struct Iter<'input, 'arena> {
    index: usize,
    indices: RefCell<Vec<usize>>,
    marker: PhantomData<&'arena Node<'input, 'arena>>,
}

impl<'input, 'arena> Iterator for Iter<'input, 'arena> {
    type Item = Ref<'input, 'arena>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.indices.borrow().get(self.index).copied().map(ptr_cast);
        self.index += 1;
        node
    }
}
