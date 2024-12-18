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

/// An opaque reference to [Node] that can be used in structs as `dyn` instead of `impl`
pub trait Ref: Debug {
    /// Upcasts the ref to `Any`
    fn inner_as_any(&self) -> &dyn std::any::Any;

    /// Creates a clone of the underlying type, usually an `Rc`
    fn clone(&self) -> Box<dyn Ref>;
}

#[cfg(not(feature = "parse"))]
#[cfg(not(feature = "serialize"))]
pub trait Features {}

#[cfg(feature = "parse")]
#[cfg(not(feature = "serialize"))]
pub trait Features: crate::parse::Node {}

#[cfg(not(feature = "parse"))]
#[cfg(feature = "serialize")]
pub trait Features: crate::serialize::Node {}

#[cfg(feature = "parse")]
#[cfg(feature = "serialize")]
pub trait Features: crate::parse::Node + crate::serialize::Node {}

/// An XML DOM node upon which other DOM API objects are based
///
/// <https://developer.mozilla.org/en-US/docs/Web/API/Node>
pub trait Node: Clone + Debug + 'static + Features {
    type Atom: Atom;
    type Child: Node<Atom = Self::Atom>;
    type ParentChild: Node<Atom = Self::Atom>;

    /// Whether the underlying pointer is at the same address as the other
    fn ptr_eq(&self, other: &impl Node) -> bool;

    /// The raw pointer address to the data
    fn as_ptr_byte(&self) -> usize;

    /// Get the node wrapped in an opaque reference
    fn as_ref(&self) -> Box<dyn Ref>;

    /// Creates a CData node with the given content
    fn c_data(&self, contents: Self::Atom) -> Self::Child;

    /// Returns an node list containing all the children of this node
    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self::Child>;

    /// Returns a read-only node list containing all the children of this node.
    ///
    /// [MDN | childNodes](https://developer.mozilla.org/en-US/docs/Web/API/Node/childNodes)
    fn child_nodes(&self) -> Vec<Self::Child>;

    /// Returns whether the node's list of children is empty or not
    fn has_child_nodes(&self) -> bool {
        self.child_nodes_iter().next().is_some()
    }

    /// Upcasts self as an element
    fn element(&self) -> Option<impl Element>;

    /// Does a breadth-first search to find an element from the current node, returning this node
    /// if it is an element.
    fn find_element(&self) -> Option<impl Element>;

    /// Returns the first child in the node's tree
    ///
    /// [MDN | firstChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/firstChild)
    fn first_child(&self) -> Option<impl Node> {
        self.child_nodes().first().map(Node::to_owned)
    }

    /// Returns the last child in the node's tree
    ///
    /// [MDN | lastChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/lastChild)
    fn last_child(&self) -> Option<impl Node> {
        self.child_nodes().last().map(Node::to_owned)
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

    /// Returns a string representing the text content of a node and it's descendants
    ///
    /// [MDN | textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)
    fn text_content(&self) -> Option<String>;

    fn set_text_content(&mut self, content: Self::Atom) {
        let text = self.text(content);
        for child in self.child_nodes_iter() {
            child.remove();
        }
        self.append_child(text);
    }

    /// Creates a text node with the given content
    fn text(&self, content: Self::Atom) -> Self::Child;

    /// Returns a [Node] that is the parent of this node. If there is no such node, like if this
    /// property if the top of the tree or if it doesn't participate in a tree, this returns [None]
    ///
    /// [MDN | parentNode](https://developer.mozilla.org/en-US/docs/Web/API/Node/parentNode)
    fn parent_node(&self) -> Option<impl Node<Child = Self::ParentChild, Atom = Self::Atom>>;

    /// Changes the return value of [Node::parent_node] to the given node
    ///
    /// # Warning
    /// This method only updated what parent it referenced, it doesn't change the child list of
    /// either the old or new parent.
    /// To avoid risking breaking the DOM tree, you must remove this element from the old parent
    /// and add it to the new parent's child list.
    ///
    /// This is intentional for a [Node] which may not need a reference to parent, but if you're
    /// using [Element], you may want to try using [Node::insert], [Node::insert_before],
    /// [Node::insert_after], [Element::after], [Element::before], or
    /// [Element::prepend]
    fn set_parent_node(
        &self,
        new_parent: &impl Node<Atom = Self::Atom>,
    ) -> Option<impl Node<Child = <Self::ParentChild as Node>::Child, Atom = Self::Atom>>;

    /// Adds a node to the end of the list of children of a specified node. This will update the
    /// parent of `a_child`
    ///
    /// [MDN | appendChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/appendChild)
    fn append_child(&mut self, a_child: Self::Child);

    /// Returns a duplicate of the node.
    ///
    /// [Spec](https://dom.spec.whatwg.org/#concept-node-clone)
    /// [MDN | cloneNode](https://developer.mozilla.org/en-US/docs/Web/API/Node/cloneNode)
    fn clone_node(&self) -> Self;

    /// Returns whether some node is a descendant if the current node.
    ///
    /// [MDN | contains](https://developer.mozilla.org/en-US/docs/Web/API/Node/contains)
    fn contains(&self, other_node: &impl Node) -> bool {
        self.child_nodes_iter().any(|c| {
            if c.ptr_eq(other_node) {
                return true;
            }
            c.contains(other_node)
        })
    }

    /// Inserts a node as the nth child of the current node's children, updating the `new_node`'s
    /// parent.
    fn insert(&mut self, index: usize, new_node: Self::Child);

    /// Inserts a node before the reference node as a child of the current node.
    ///
    /// [MDN | insertBefore](https://developer.mozilla.org/en-US/docs/Web/API/Node/insertBefore)
    fn insert_before(&mut self, new_node: Self::Child, reference_node: Self::Child) {
        let len = self.child_nodes().len();
        let reference_index = self.child_index(reference_node).unwrap_or(len);
        self.insert(reference_index - 1, new_node);
    }

    /// Inserts a node after the reference node as a child of the current node.
    ///
    /// [MDN | insertAfter](https://developer.mozilla.org/en-US/docs/Web/API/Node/insertAfter)
    fn insert_after(&mut self, new_node: Self::Child, reference_node: Self::Child) {
        let len = self.child_nodes().len();
        let reference_index = self.child_index(reference_node).unwrap_or(len - 2);
        self.insert(reference_index + 1, new_node);
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
        let child_index = self
            .child_nodes_iter()
            .enumerate()
            .find(|(_, n)| n.ptr_eq(&child))
            .map(|(i, _)| i)?;
        self.remove_child_at(child_index)
    }

    /// Replaces a child node with the given one
    ///
    /// Note that the argument order in the spec is unusual, [Element::replace_with] may be easier
    /// to follow
    ///
    /// [MDN | replaceChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/replaceChild)
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

    /// Returns the index of the child within the current node's child list
    fn child_index(&self, child: Self::Child) -> Option<usize> {
        self.child_nodes()
            .iter()
            .enumerate()
            .find(|(_, n)| n.ptr_eq(&child))
            .map(|(i, _)| i)
    }

    /// Create a cloned refcell without copying the underlying data
    fn to_owned(&self) -> Self;

    /// Upcast the node as an `impl Node`
    fn as_impl(&self) -> impl Node;

    fn as_child(&self) -> Self::Child;

    /// Upcast the node as the specified `ParentChild`
    fn as_parent_child(&self) -> Self::ParentChild;
}
