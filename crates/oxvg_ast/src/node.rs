//! XML node traits.
use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
};

use itertools::Itertools;
use lightningcss::{printer::PrinterOptions, rules::CssRuleList};

use crate::{
    arena::Arena,
    atom::Atom,
    attribute::data::Attr,
    element::{data::ElementId, Element},
    serialize::ToAtom as _,
};

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
    /// The text inside a style element
    Style,
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

#[derive(derive_more::Debug, Clone)]
/// The data of a node in an XML document.
pub enum NodeData<'input> {
    /// The document.
    Document,
    /// The root of a document, contains the root element, doctype, PIs, etc.
    Root,
    /// An element. (e.g. <a xlink:href="#">hello</a>)
    Element {
        /// The qualified name of the element's tag.
        name: ElementId<'input>,
        /// The attributes of the element.
        attrs: RefCell<Vec<Attr<'input>>>,
        #[cfg(feature = "selectors")]
        #[debug(skip)]
        /// Flags used for caching whether an element matches a selector
        selector_flags: Cell<Option<selectors::matching::ElementSelectorFlags>>,
    },
    /// A processing instruction. (e.g. <?xml version="1.0"?>)
    PI {
        /// The name of the application to which the instruction is targeted
        target: Atom<'input>,
        /// Data for the application
        value: RefCell<Option<Atom<'input>>>,
    },
    /// A comment node. (e.g. `<!-- foo ->`)
    Comment(RefCell<Option<Atom<'input>>>),
    /// A text node. (e.g. `foo` of `<p>foo</p>`)
    Text(RefCell<Option<Atom<'input>>>),
    /// A text node of a style element. (e.g. `a { color: blue; }` of `<style>a { color: blue; }</style>`)
    Style(CssRuleList<'input>),
}

struct ChildNodes<'arena> {
    front: Option<Ref<'arena>>,
    front_next: Option<Ref<'arena>>,
    end_previous: Option<Ref<'arena>>,
    end: Option<Ref<'arena>>,
}

impl<'arena> Iterator for ChildNodes<'arena> {
    type Item = Ref<'arena>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.front?;

        // Move front tracking forwards
        let new_front_next = self.front_next.and_then(|n| n.next_sibling());
        self.front = std::mem::replace(&mut self.front_next, new_front_next);

        // End iteration when it collides with end
        if self.end.is_some_and(|end| end == current) {
            self.front = None;
            self.front_next = None;
            self.end_previous = None;
            self.end = None;
        }

        // Done
        Some(current)
    }
}

impl DoubleEndedIterator for ChildNodes<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let current = self.end?;

        let new_end_previous = self.end_previous.and_then(|n| n.next_sibling());
        self.end = std::mem::replace(&mut self.end_previous, new_end_previous);

        if self.front.is_some_and(|front| front == current) {
            self.front = None;
            self.front_next = None;
            self.end_previous = None;
            self.end = None;
        }

        Some(current)
    }
}

impl ExactSizeIterator for ChildNodes<'_> {
    fn len(&self) -> usize {
        let mut current = self.front;
        let mut len = 0;
        while let Some(node) = current {
            current = node.next_sibling();
            len += 1;
            if self.end.is_none_or(|end| end == node) {
                break;
            }
        }
        len
    }
}

#[derive(Clone)]
/// An XML DOM node upon which other DOM API objects are based
///
/// [MDN | Node](https://developer.mozilla.org/en-US/docs/Web/API/Node)
pub struct Node<'arena> {
    /// The node's parent.
    pub parent: Link<'arena>,
    /// The node before this of the node's parent's children
    pub next_sibling: Link<'arena>,
    /// The node after this of the node's parent's children
    pub previous_sibling: Link<'arena>,
    /// The node's first child.
    pub first_child: Link<'arena>,
    /// The node's last child.
    pub last_child: Link<'arena>,
    /// The node's type and associated data.
    pub node_data: NodeData<'arena>,
    /// The node's id, determined by it's allocation
    id: usize,
}

/// A reference to a node
pub type Ref<'arena> = &'arena Node<'arena>;
/// A settable reference to a node
pub type Link<'arena> = Cell<Option<Ref<'arena>>>;

impl<'arena> Node<'arena> {
    fn text_content_recursive(&self) -> Option<Atom<'arena>> {
        match &self.node_data {
            NodeData::Text(value) | NodeData::Comment(value) | NodeData::PI { value, .. } => {
                value.borrow().clone()
            }
            NodeData::Style(style) => style
                .to_atom_string(PrinterOptions::default())
                .map(Into::into)
                .ok(),
            NodeData::Document | NodeData::Root => None,
            NodeData::Element { .. } => Some(
                self.child_nodes_iter()
                    .filter_map(Self::text_content_recursive)
                    .fold(String::default(), |mut acc, item| {
                        acc.push_str(&item);
                        acc
                    })
                    .into(),
            ),
        }
    }

    /// Creates a clean node with the given node data.
    pub fn new(node_data: NodeData<'arena>, id: usize) -> Self {
        Self {
            parent: Cell::new(None),
            next_sibling: Cell::new(None),
            previous_sibling: Cell::new(None),
            first_child: Cell::new(None),
            last_child: Cell::new(None),
            node_data,
            id,
        }
    }

    /// Whether the allocation id is the same address as the other
    pub fn id_eq(&self, other: &Node<'_>) -> bool {
        self.id() == other.id()
    }

    /// The allocation id
    fn id(&self) -> usize {
        self.id
    }

    /// Returns an node list containing all the children of this node
    pub fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Ref<'arena>> {
        ChildNodes {
            front: self.first_child(),
            front_next: self.first_child().and_then(|n| n.next_sibling()),
            end_previous: self.last_child().and_then(|n| n.previous_sibling()),
            end: self.last_child(),
        }
    }

    /// Returns the number of child nodes by iteration
    pub fn child_node_count(&self) -> usize {
        self.child_nodes_iter().count()
    }

    /// Returns whether the node's list of children is empty or not
    pub fn has_child_nodes(&self) -> bool {
        self.child_node_count() > 0
    }

    /// Upcasts self as an element
    pub fn element(&'arena self) -> Option<Element<'arena>> {
        match self.node_type() {
            Type::Element | Type::Document => Element::new(self),
            _ => None,
        }
    }

    /// Removes all child nodes
    fn empty(&self) {
        self.first_child.set(None);
        self.last_child.set(None);
    }

    /// Does a breadth-first search to find an element from the current node, returning this node
    /// if it is an element.
    pub fn find_element(&'arena self) -> Option<Element<'arena>> {
        Element::find_element(self)
    }

    /// Returns the first child in the node's tree
    ///
    /// [MDN | firstChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/firstChild)
    fn first_child(&self) -> Option<Ref<'arena>> {
        self.first_child.get()
    }

    /// Inserts a node before the reference node as a child of the current node.
    ///
    /// [MDN | insertBefore](https://developer.mozilla.org/en-US/docs/Web/API/Node/insertBefore)
    pub fn insert_before(&'arena self, new_node: Ref<'arena>, reference_node: Ref<'arena>) {
        new_node.remove();
        new_node.parent.set(Some(self));
        let Some(prev_child) = reference_node.previous_sibling.replace(Some(new_node)) else {
            self.first_child.set(Some(new_node));
            new_node.next_sibling.set(Some(reference_node));
            return;
        };
        prev_child.next_sibling.set(Some(new_node));
        new_node.previous_sibling.set(Some(prev_child));
        new_node.next_sibling.set(Some(reference_node));
        debug_assert!(new_node.parent.get() == Some(self));
        debug_assert!(new_node.next_sibling.get() == Some(reference_node));
        debug_assert!(reference_node.previous_sibling.get() == Some(new_node));
    }

    /// Inserts a node after the reference node as a child of the current node.
    ///
    /// [MDN | insertAfter](https://developer.mozilla.org/en-US/docs/Web/API/Node/insertAfter)
    pub fn insert_after(&'arena self, new_node: Ref<'arena>, reference_node: &Ref<'arena>) {
        new_node.remove();
        new_node.parent.set(Some(self));
        let Some(next_child) = reference_node.next_sibling.replace(Some(new_node)) else {
            self.last_child.set(Some(new_node));
            new_node.previous_sibling.set(Some(reference_node));
            return;
        };
        next_child.previous_sibling.set(Some(new_node));
        new_node.next_sibling.set(Some(next_child));
        new_node.previous_sibling.set(Some(reference_node));
        debug_assert!(new_node.parent.get() == Some(self));
        debug_assert!(new_node.previous_sibling.get() == Some(*reference_node));
        debug_assert!(reference_node.next_sibling.get() == Some(new_node));
    }

    /// Iterates through the children of the node, using the callback to determine which
    /// of the nodes to remove
    fn retain_children<F>(&self, mut f: F)
    where
        F: FnMut(Ref<'arena>) -> bool,
    {
        self.last_child.set(None);
        let mut current = self.first_child.take();
        let mut previously_retained = None;
        while let Some(child) = current {
            current = child.next_sibling.get();
            let retain = f(child);
            if retain {
                child.previous_sibling.set(previously_retained);
                if previously_retained.is_none() {
                    self.first_child.set(Some(child));
                }
                previously_retained = Some(child);
                self.last_child.set(Some(child));
            } else {
                child.parent.set(None);
                child.previous_sibling.set(None);
                child.next_sibling.set(None);
            }
        }
    }

    /// Returns the last child in the node's tree
    ///
    /// [MDN | lastChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/lastChild)
    pub fn last_child(&self) -> Option<Ref<'arena>> {
        self.last_child.get()
    }

    /// Returns the node immediately following itself from the parent's list of children
    ///
    /// [MDN | nextSibling](https://developer.mozilla.org/en-US/docs/Web/API/Node/nextSibling)
    fn next_sibling(&self) -> Option<Ref<'arena>> {
        self.next_sibling.get()
    }

    /// Returns an enum that identifies what the node is.
    ///
    /// [MDN | nodeType](https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeType)
    pub fn node_type(&self) -> Type {
        self.node_data.node_type()
    }

    /// Returns the node immediately before itself from the parent's list of children
    ///
    /// [MDN | previousSibling](https://developer.mozilla.org/en-US/docs/Web/API/Node/previousSibling)
    fn previous_sibling(&self) -> Option<Ref<'arena>> {
        self.previous_sibling.get()
    }

    /// Returns a [Node] that is the parent of this node. If there is no such node, like if this
    /// property if the top of the tree or if it doesn't participate in a tree, this returns [None]
    ///
    /// [MDN | parentNode](https://developer.mozilla.org/en-US/docs/Web/API/Node/parentNode)
    pub fn parent_node(&self) -> Option<Element<'arena>> {
        self.parent.get().and_then(Element::new)
    }

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
    pub fn set_parent_node(&'arena self, new_parent: &Element<'arena>) -> Option<Element<'arena>> {
        self.parent
            .replace(Some(new_parent.0))
            .and_then(Element::new)
    }

    /// Adds a node to the end of the list of children of a specified node. This will update the
    /// parent of `a_child`
    ///
    /// [MDN | appendChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/appendChild)
    pub fn append_child(&'arena self, a_child: Ref<'arena>) {
        a_child.parent.set(Some(self));
        if let Some(child) = self.last_child.replace(Some(a_child)) {
            child.next_sibling.set(Some(a_child));
            a_child.previous_sibling.set(Some(child));
        } else {
            self.first_child.set(Some(a_child));
        }
        debug_assert!(a_child.parent.get() == Some(self));
        debug_assert!(a_child.next_sibling.get().is_none());
        debug_assert!(self.last_child.get() == Some(a_child));
    }

    /// Returns a node from the child nodes
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NodeList/item)
    fn item(&self, index: usize) -> Option<Ref<'arena>> {
        self.child_nodes_iter().nth(index)
    }

    /// Returns whether the node has zero child nodes
    fn is_empty(&self) -> bool {
        self.first_child().is_none_or(|_| {
            self.child_nodes_iter().all(|n| {
                n.node_type() == Type::Text && n.text_content().is_none_or(|t| t.trim().is_empty())
            })
        })
    }

    /// Returns a string containins the name of the [Node]. The structure of the name will differ
    /// with the node type. E.g. An `Element` will contain the name of the corresponding tag, like
    /// `"AUDIO"` for an `HTMLAudioElement`, a text node will have the `"#text"` string, or a
    /// `Document` node will have the `"#document"` string.
    ///
    /// [MDN | nodeName](https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeName)
    pub fn node_name(&self) -> Atom<'arena> {
        self.node_data.name()
    }

    /// Returns a string containing the value of the node.
    ///
    /// [MDN | nodeValue](https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeValue)
    fn node_value(&self) -> Option<Atom<'arena>> {
        self.node_data.value()
    }

    /// Returns the processing instruction's target and data, if the node is a processing
    /// instruction
    pub fn processing_instruction(&self) -> Option<(Atom<'arena>, Atom<'arena>)> {
        self.node_data.processing_instruction()
    }

    /// Tries settings the value of the node if possible. If not possible, returns [None].
    ///
    /// Setting a node's value is only possible for node types which can return a node value:
    /// - `CDataSection`
    /// - `Comment`
    /// - `ProcessingInstruction`
    /// - `Text`
    ///
    /// However, depending on implementation details these types may be read-only
    fn try_set_node_value(&self, value: Atom<'arena>) -> Option<()> {
        self.node_data.try_set_node_value(value)
    }

    /// Returns a string representing the text content of a node and it's descendants
    ///
    /// [MDN | textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)
    pub fn text_content(&self) -> Option<Atom<'arena>> {
        if !self.is_empty() {
            return self.text_content_recursive();
        }
        match &self.node_data {
            NodeData::Document | NodeData::Root => None,
            NodeData::Text(value) | NodeData::Comment(value) | NodeData::PI { value, .. } => {
                value.borrow().clone()
            }
            NodeData::Style(style) => style
                .to_atom_string(PrinterOptions::default())
                .map(Into::into)
                .ok(),
            NodeData::Element { .. } => Some(Atom::default()),
        }
    }

    /// Replaces all child nodes with a text node of the given content
    fn set_text_content(&'arena self, content: Atom<'arena>, arena: &Arena<'arena>) {
        match self.node_data {
            NodeData::Text(ref value) => {
                value.replace(Some(content));
            }
            NodeData::Element { .. } => {
                self.empty();
                self.append_child(self.text(content, arena));
            }
            _ => {}
        }
    }

    /// Creates a text node with the given content
    fn text(&self, content: Atom<'arena>, arena: &Arena<'arena>) -> Ref<'arena> {
        arena.alloc(Node::new(
            NodeData::Text(RefCell::new(Some(content))),
            arena.len(),
        ))
    }

    /// Removes the current node from it's parent and removes the reference to the parent
    ///
    /// Note, this element is usually reserved for [Element], but is available for [Node] if
    /// needed.
    ///
    /// [MDN | remove](https://developer.mozilla.org/en-US/docs/Web/API/Element/remove)
    pub fn remove(&self) {
        let parent = self.parent.take();
        let previous_sibling = self.previous_sibling.take();
        let next_sibling = self.next_sibling.take();
        if let Some(previous_sibling) = previous_sibling {
            if let Some(next_sibling) = next_sibling {
                // prev -> ~self~ -> next
                next_sibling.previous_sibling.set(Some(previous_sibling));
            } else if let Some(parent) = parent {
                // prev -> ~self~ -> None
                parent.last_child.set(Some(previous_sibling));
            }
            previous_sibling.next_sibling.set(next_sibling);
        } else if let Some(next_sibling) = next_sibling {
            next_sibling.previous_sibling.set(None);
            if let Some(parent) = parent {
                // None -> ~self~ -> next
                parent.first_child.set(Some(next_sibling));
            }
        } else if let Some(parent) = parent {
            // None -> ~self~ -> None
            parent.first_child.set(None);
            parent.last_child.set(None);
        }
        debug_assert!(previous_sibling.is_none_or(|n| n.next_sibling.get() == next_sibling));
        debug_assert!(next_sibling.is_none_or(|n| n.previous_sibling.get() == previous_sibling));
        debug_assert!(parent.is_none_or(|n| n.first_child.get() != Some(self)));
        debug_assert!(parent.is_none_or(|n| n.last_child.get() != Some(self)));
    }

    /// Remove the nth child from this node's child list
    fn remove_child_at(&mut self, index: usize) -> Option<Ref<'arena>> {
        let child = self.child_nodes_iter().nth(index);
        child?.remove();
        child
    }

    /// Returns a duplicate of the node.
    ///
    /// [Spec](https://dom.spec.whatwg.org/#concept-node-clone)
    /// [MDN | cloneNode](https://developer.mozilla.org/en-US/docs/Web/API/Node/cloneNode)
    fn clone_node(&self) -> Self {
        todo!("needs arena")
    }

    /// Returns whether some node is a descendant if the current node.
    ///
    /// [MDN | contains](https://developer.mozilla.org/en-US/docs/Web/API/Node/contains)
    fn contains(&self, other_node: &Node<'arena>) -> bool {
        self.child_nodes_iter().any(|c| {
            if c.id_eq(other_node) {
                return true;
            }
            c.contains(other_node)
        })
    }

    /// Inserts a node as the nth child of the current node's children, updating the `new_node`'s
    /// parent.
    fn insert(&'arena self, index: usize, new_node: Ref<'arena>) {
        if index == 0 {
            if let Some(first_child) = self.first_child() {
                self.insert_before(new_node, &first_child);
            } else {
                self.append_child(new_node);
            }
        } else if let Some(prev_child) = self.item(index - 1) {
            self.insert_after(new_node, &prev_child);
        } else {
            self.append_child(new_node);
        }
    }

    /// Returns the index of the child within the current node's child list
    fn child_index(&self, child: &Ref<'arena>) -> Option<usize> {
        let mut result = None;
        let mut index = 0;
        self.child_nodes_iter().any(|sibling| {
            if sibling.id_eq(child) {
                result = Some(index);
                true
            } else {
                index += 1;
                false
            }
        });
        result
    }

    /// Removes a child node from this node's child list
    ///
    /// [MDN | removeChild](https://developer.mozilla.org/en-US/docs/Web/API/Node/removeChild)
    fn remove_child(&mut self, child: Ref<'arena>) -> Option<Ref<'arena>> {
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
        &'arena self,
        new_child: Ref<'arena>,
        old_child: &Ref<'arena>,
    ) -> Option<Ref<'arena>> {
        debug_assert_eq!(old_child.parent.get(), Some(self));
        debug_assert!(self.child_nodes_iter().contains(old_child));

        let previous_sibling = old_child.previous_sibling.take();
        let next_sibling = old_child.next_sibling.take();
        old_child.parent.set(None);

        new_child.previous_sibling.set(previous_sibling);
        new_child.next_sibling.set(next_sibling);
        new_child.parent.set(Some(self));

        if let Some(previous_sibling) = previous_sibling {
            previous_sibling.next_sibling.set(Some(new_child));
        } else {
            self.first_child.set(Some(new_child));
        }
        if let Some(next_sibling) = next_sibling {
            next_sibling.previous_sibling.set(Some(new_child));
        } else {
            self.last_child.set(Some(new_child));
        }
        Some(*old_child)
    }
}

impl Eq for Node<'_> {}

impl PartialEq for Node<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl std::hash::Hash for Node<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = &self.node_data;
        let Node {
            last_child, parent, ..
        } = self;
        let parent = parent.get().is_some();
        f.debug_struct("Node")
            .field("data", data)
            .field("children", &self.child_nodes_iter().collect_vec())
            .field("last_child", last_child)
            .field("has_parent", &parent)
            .finish()
    }
}

impl<'input> NodeData<'input> {
    fn node_type(&self) -> Type {
        match self {
            Self::Root | Self::Document => Type::Document,
            Self::Element { .. } => Type::Element,
            Self::PI { .. } => Type::ProcessingInstruction,
            Self::Text { .. } => Type::Text,
            Self::Style(..) => Type::Style,
            Self::Comment(..) => Type::Comment,
        }
    }

    fn name(&self) -> Atom<'input> {
        match self {
            Self::Comment { .. } => "#comment".into(),
            Self::Document | Self::Root => "#document".into(),
            Self::Element { name, .. } => name.local_name().to_uppercase().into(),
            Self::PI { target, .. } => target.clone(),
            Self::Text { .. } | Self::Style(..) => "#text".into(),
        }
    }

    fn value(&self) -> Option<Atom<'input>> {
        match &self {
            Self::Comment(value) | Self::Text(value) | Self::PI { value, .. } => {
                value.borrow().clone()
            }
            _ => None,
        }
    }

    fn processing_instruction(&self) -> Option<(Atom<'input>, Atom<'input>)> {
        match self {
            NodeData::PI { target, value } => {
                Some((target.clone(), value.borrow().as_ref().unwrap().clone()))
            }
            _ => None,
        }
    }

    fn try_set_node_value(&self, value: Atom<'input>) -> Option<()> {
        match self {
            Self::Text(old_value) => {
                old_value.replace(Some(value));
                Some(())
            }
            _ => None,
        }
    }
}
