use crate::{atom::Atom, element::Element};

#[cfg(feature = "parse")]
use crate::parse;

#[cfg(feature = "serialize")]
use crate::serialize;

#[derive(PartialEq)]
pub enum Type {
    Element,
    Attribute,
    Text,
    CDataSection,
    ProcessingInstruction,
    Comment,
    Document,
    DocumentType,
    DocumentFragment,
}

pub trait Node: Sized + 'static + parse::Node + serialize::Node {
    type Atom: Atom;
    type Child: Node<Atom = Self::Atom>;
    type ParentChild: Node<Atom = Self::Atom>;

    /// Whether the underlying pointer is at the same address as the other
    fn ptr_eq(&self, other: &impl Node) -> bool;

    /// Returns an node list containing all the children of this node
    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self>;

    /// Returns an node list containing all the children of this node
    fn child_nodes(&self) -> Vec<Self::Child>;

    /// Upcasts self as an element
    fn element(&self) -> Option<impl Element>;

    fn first_child(&self) -> Option<impl Node> {
        self.child_nodes().first().map(Node::to_owned)
    }

    fn last_child(&self) -> Option<impl Node> {
        self.child_nodes().last().map(Node::to_owned)
    }

    fn next_sibling(&self) -> Option<Self::ParentChild> {
        self.parent_node()?
            .child_nodes()
            .iter()
            .take_while(|n| !n.ptr_eq(self))
            .next()
            .map(Node::to_owned)
    }

    /// Returns a string containins the name of the [Node]. The structure of the name will differ
    /// with the node type. E.g. An `Element` will contain the name of the corresponding tag, like
    /// `"AUDIO"` for an `HTMLAudioElement`, a text node will have the `"#text"` string, or a
    /// `Document` node will have the `"#document"` string.
    fn node_name(&self) -> Self::Atom;

    fn node_type(&self) -> Type;

    fn node_value(&self) -> Option<Self::Atom>;

    /// Returns a [Node] that is the parent of this node. If there is no such node, like if this
    /// property if the top of the tree or if it doesn't participate in a tree, this returns [None]
    fn parent_node(&self) -> Option<impl Node<Child = Self::ParentChild, Atom = Self::Atom>>;

    fn append_child(&mut self, a_child: Self::Child) {
        self.child_nodes().push(a_child);
    }

    /// <https://dom.spec.whatwg.org/#concept-node-clone>
    fn clone_node(&self) -> Self;

    fn contains(&self, other_node: &Self::Child) -> bool {
        self.child_nodes_iter().any(|c| {
            if c.ptr_eq(other_node) {
                return true;
            }
            c.contains(other_node)
        })
    }

    fn has_child_nodes(&self) -> bool {
        !self.child_nodes().is_empty()
    }

    fn insert_before(&mut self, new_node: Self::Child, reference_node: Self::Child) {
        let len = self.child_nodes().len();
        let reference_index = self.child_index(reference_node).unwrap_or(len);
        self.child_nodes().insert(reference_index - 1, new_node);
    }

    fn insert_after(&mut self, new_node: Self::Child, reference_node: Self::Child) {
        let len = self.child_nodes().len();
        let reference_index = self.child_index(reference_node).unwrap_or(len - 2);
        self.child_nodes().insert(reference_index + 1, new_node);
    }

    fn remove_child(&mut self, child: Self::Child) -> Option<Self::Child> {
        let mut children = self.child_nodes();
        let child_index = children
            .iter()
            .enumerate()
            .find(|(_, n)| n.ptr_eq(&child))
            .map(|(i, _)| i);
        child_index.map(|i| children.remove(i))
    }

    fn replace_child(
        &mut self,
        new_child: Self::Child,
        old_child: Self::Child,
    ) -> Option<Self::Child> {
        let mut children = self.child_nodes();
        Some(std::mem::replace(
            &mut children[self.child_index(old_child)?],
            new_child,
        ))
    }

    fn child_index(&self, child: Self::Child) -> Option<usize> {
        self.child_nodes()
            .iter()
            .enumerate()
            .find(|(_, n)| n.ptr_eq(&child))
            .map(|(i, _)| i)
    }

    /// Create a cloned refcell without copying the underlying data
    fn to_owned(&self) -> Self;

    fn as_impl(&self) -> impl Node;

    fn as_parent_child(&self) -> Self::ParentChild;
}
