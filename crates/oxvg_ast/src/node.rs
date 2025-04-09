//! XML node traits.
use std::fmt::Debug;

use crate::{atom::Atom, element::Element};

#[derive(PartialEq, Debug)]
/// An enum which specifies the type of node.
///
/// # Notes
///
/// * that normally the type would be represented as a number
/// * The following deprecated types are not included
///   * `EntityReferenceNode`
///   * `EntityNode`
///   * `NotationNode`
///
/// [MDN | nodeType](https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeType)
pub enum Type {
    /// An [Element] node like `<p>` or `<div>`
    Element,
    /// An `Attribute` of an [Element]
    Attribute,
    /// The actual text inside an element or attribute
    Text,
    /// A `CDataSection`, such as `<!CDATA[[ ... ]]>`
    CDataSection,
    /// A `ProcessingInstruction` of an XML document, such as `<?xml-stylesheet ... ?>`
    ProcessingInstruction,
    /// A `Comment` node, such as `<!-- ... -->`
    Comment,
    /// A `Document` node
    Document,
    /// A `DocumentType` node such as `<!doctype html>`
    DocumentType,
    /// A `DocumentFragment` node
    DocumentFragment,
}

/// An XML DOM node upon which other DOM API objects are based
///
/// [MDN | Node](https://developer.mozilla.org/en-US/docs/Web/API/Node)
pub trait Node<'arena>: Clone + Debug {
    /// The type of an allocator for a node. This may be `()` for implementations
    /// not using an allocator
    type Arena;
    /// The text type of a node's content
    type Atom: Atom;
    /// The node type of the child of a node
    type Child: Node<'arena, Atom = Self::Atom, Arena = Self::Arena>;
    /// The node type of the sibling of a node
    type ParentChild: Node<'arena, Atom = Self::Atom, Parent = Self::Parent, Arena = Self::Arena>;
    /// The node type of the parent of a node
    type Parent: Node<'arena, Atom = Self::Atom, Child = Self::ParentChild, Arena = Self::Arena>;

    /// Whether the underlying pointer is at the same address as the other
    fn ptr_eq(&self, other: &impl Node<'arena>) -> bool;

    /// The raw pointer address to the data
    fn as_ptr_byte(&self) -> usize;

    /// Returns an node list containing all the children of this node
    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self::Child>;

    /// Returns a read-only node list containing all the children of this node.
    ///
    /// [MDN | childNodes](https://developer.mozilla.org/en-US/docs/Web/API/Node/childNodes)
    fn child_nodes(&self) -> Vec<Self::Child> {
        self.child_nodes_iter().collect()
    }

    /// Returns the number of child nodes by iteration
    fn child_node_count(&self) -> usize {
        self.child_nodes_iter().count()
    }

    /// Returns whether the node's list of children is empty or not
    fn has_child_nodes(&self) -> bool {
        self.child_node_count() > 0
    }

    /// Upcasts self as an element
    fn element(&self) -> Option<impl Element<'arena>>;

    /// Removes all child nodes
    fn empty(&self);

    /// Does a breadth-first search to find an element from the current node, returning this node
    /// if it is an element.
    fn find_element(&self) -> Option<impl Element<'arena>>;

    /// Returns the first child in the node's tree
    ///
    /// [MDN | firstChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/firstChild)
    fn first_child(&self) -> Option<Self::Child> {
        self.child_nodes().first().cloned()
    }

    /// Iterates through the children of the node, using the callback to determine which
    /// of the nodes to remove
    fn retain_children<F>(&self, f: F)
    where
        F: FnMut(Self::Child) -> bool;

    /// Returns the last child in the node's tree
    ///
    /// [MDN | lastChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/lastChild)
    fn last_child(&self) -> Option<Self::Child> {
        self.child_nodes().last().cloned()
    }

    /// Returns the node immediately following itself from the parent's list of children
    ///
    /// [MDN | nextSibling](https://developer.mozilla.org/en-US/docs/Web/API/Node/nextSibling)
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
    ///
    /// [MDN | nodeName](https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeName)
    fn node_name(&self) -> Self::Atom;

    /// Returns an enum that identifies what the node is.
    ///
    /// [MDN | nodeType](https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeType)
    fn node_type(&self) -> Type;

    /// Returns a string containing the value of the node.
    ///
    /// [MDN | nodeValue](https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeValue)
    fn node_value(&self) -> Option<Self::Atom>;

    /// Returns the processing instruction's target and data, if the node is a processing
    /// instruction
    fn processing_instruction(&self) -> Option<(Self::Atom, Self::Atom)>;

    /// Tries settings the value of the node if possible. If not possible, returns [None].
    ///
    /// Setting a node's value is only possible for node types which can return a node value:
    /// - `CDataSection`
    /// - `Comment`
    /// - `ProcessingInstruction`
    /// - `Text`
    ///
    /// However, depending on implementation details these types may be read-only
    fn try_set_node_value(&self, value: Self::Atom) -> Option<()>;

    /// Returns a string representing the text content of a node and it's descendants
    ///
    /// [MDN | textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)
    fn text_content(&self) -> Option<Self::Atom>;

    /// Replaces all child nodes with a text node of the given content
    fn set_text_content(&self, content: Self::Atom, arena: &Self::Arena);

    /// Creates a text node with the given content
    fn text(&self, content: Self::Atom, arena: &Self::Arena) -> Self::Child;

    /// Returns a [Node] that is the parent of this node. If there is no such node, like if this
    /// property if the top of the tree or if it doesn't participate in a tree, this returns [None]
    ///
    /// [MDN | parentNode](https://developer.mozilla.org/en-US/docs/Web/API/Node/parentNode)
    fn parent_node(&self) -> Option<Self::Parent>;

    /// Changes the return value of [`Node::parent_node`] to the given node
    ///
    /// # Warning
    /// This method only updated what parent it referenced, it doesn't change the child list of
    /// either the old or new parent.
    /// To avoid risking breaking the DOM tree, you must remove this element from the old parent
    /// and add it to the new parent's child list.
    ///
    /// This is intentional for a [Node] which may not need a reference to parent, but if you're
    /// using [Element], you may want to try using [`Node::insert`], [`Node::insert_before`],
    /// [`Node::insert_after`], [`Element::after`], [`Element::before`], or
    /// [`Element::prepend`]
    fn set_parent_node(&self, new_parent: &Self::Parent) -> Option<Self::Parent>;

    /// Adds a node to the end of the list of children of a specified node. This will update the
    /// parent of `a_child`
    ///
    /// [MDN | appendChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/appendChild)
    fn append_child(&self, a_child: Self::Child);

    /// Returns a duplicate of the node.
    ///
    /// [Spec](https://dom.spec.whatwg.org/#concept-node-clone)
    /// [MDN | cloneNode](https://developer.mozilla.org/en-US/docs/Web/API/Node/cloneNode)
    fn clone_node(&self) -> Self;

    /// Returns whether some node is a descendant if the current node.
    ///
    /// [MDN | contains](https://developer.mozilla.org/en-US/docs/Web/API/Node/contains)
    fn contains(&self, other_node: &impl Node<'arena>) -> bool {
        self.child_nodes_iter().any(|c| {
            if c.as_impl().ptr_eq(other_node) {
                return true;
            }
            c.contains(other_node)
        })
    }

    /// Inserts a node as the nth child of the current node's children, updating the `new_node`'s
    /// parent.
    fn insert(&mut self, index: usize, new_node: Self::Child) {
        if let Some(prev_child) = self.item(index) {
            self.insert_after(new_node, &prev_child);
        } else {
            self.append_child(new_node);
        }
    }

    /// Inserts a node before the reference node as a child of the current node.
    ///
    /// [MDN | insertBefore](https://developer.mozilla.org/en-US/docs/Web/API/Node/insertBefore)
    fn insert_before(&mut self, new_node: Self::Child, reference_node: &Self::Child) {
        let len = self.child_nodes().len();
        let reference_index = self.child_index(reference_node).unwrap_or(len);
        self.insert(reference_index - 1, new_node);
    }

    /// Inserts a node after the reference node as a child of the current node.
    ///
    /// [MDN | insertAfter](https://developer.mozilla.org/en-US/docs/Web/API/Node/insertAfter)
    fn insert_after(&mut self, new_node: Self::Child, reference_node: &Self::Child) {
        let len = self.child_nodes().len();
        let reference_index = self
            .child_index(reference_node)
            .unwrap_or(len.saturating_sub(2));
        self.insert(reference_index + 1, new_node);
    }

    /// Returns a node from the child nodes
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NodeList/item)
    fn item(&self, index: usize) -> Option<Self::Child>;

    /// Returns whether the node has zero child nodes
    fn is_empty(&self) -> bool {
        self.first_child().is_none()
    }

    /// Removes the current node from it's parent and removes the reference to the parent
    ///
    /// Note, this element is usually reserved for [Element], but is available for [Node] if
    /// needed.
    ///
    /// [MDN | remove](https://developer.mozilla.org/en-US/docs/Web/API/Element/remove)
    fn remove(&self);

    /// Remove the nth child from this node's child list
    fn remove_child_at(&mut self, index: usize) -> Option<Self::Child>;

    /// Removes a child node from this node's child list
    ///
    /// [MDN | removeChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/removeChild)
    fn remove_child(&mut self, child: Self::Child) -> Option<Self::Child> {
        let child_index = self.child_index(&child)?;
        self.remove_child_at(child_index)
    }

    /// Replaces a child node with the given one
    ///
    /// Note that the argument order in the spec is unusual, [`Element::replace_with`] may be easier
    /// to follow
    ///
    /// [MDN | replaceChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/replaceChild)
    fn replace_child(
        &mut self,
        new_child: Self::Child,
        old_child: &Self::Child,
    ) -> Option<Self::Child>;

    /// Returns the index of the child within the current node's child list
    fn child_index(&self, child: &Self::Child) -> Option<usize> {
        let mut result = None;
        let mut index = 0;
        self.child_nodes_iter().any(|sibling| {
            if sibling.ptr_eq(child) {
                result = Some(index);
                true
            } else {
                index += 1;
                false
            }
        });
        result
    }

    /// Create a cloned refcell without copying the underlying data
    fn to_owned(&self) -> Self;

    /// Upcast the node as an `impl Node`
    fn as_impl(&self) -> impl Node<'arena>;

    /// Upcase the node as the specified `Child`
    fn as_child(&self) -> Self::Child;

    /// Upcast the node as the specified `ParentChild`
    fn as_parent_child(&self) -> Self::ParentChild;
}
