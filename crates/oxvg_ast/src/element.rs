//! XML element traits.
use std::{
    cell::{self, Cell, RefCell, RefMut},
    collections::VecDeque,
    fmt::Debug,
    ops::Deref,
};

use cfg_if::cfg_if;
use itertools::Itertools as _;

use crate::{
    arena::Allocator,
    atom::Atom,
    attribute::{
        data::{Attr, AttrId},
        Attributes,
    },
    class_list::ClassList,
    document::Document,
    element::data::Iterator,
    name::{Prefix, QualName, NS},
    node::{self, NodeData, Ref},
};
use data::ElementId;

pub mod category;

pub mod data;

#[macro_export]
/// Returns whether the element matches the given names
///
/// Returns whether the element is an element, when no names are given
macro_rules! is_element {
    ($element:expr) => {
        $element.node_type() == $crate::node::Type::Element
    };
    ($element:expr, $name:ident$(,)?) => {
        *$element.qual_name() == $crate::element::data::ElementId::$name
    };
    ($element:expr, $($name:ident)|+$(,)?) => {
        matches!($element.qual_name().unaliased(), $($crate::element::data::ElementId::$name)|+)
    };
}

#[derive(Clone, Eq)]
/// An XML element type.
#[repr(transparent)]
pub struct Element<'input, 'arena>(pub Ref<'input, 'arena>);

/// A hashable wrapper of [`Element`] that hashes based on it's allocation id.
///
/// Note that hash collisions are likely when using elements from seperate arenas.
#[derive(Clone, Debug)]
pub struct HashableElement<'input, 'arena>(Element<'input, 'arena>);
impl<'input, 'arena> HashableElement<'input, 'arena> {
    /// Creates a new hashable element
    pub fn new(element: Element<'input, 'arena>) -> Self {
        Self(element)
    }
}
impl<'input, 'arena> Deref for HashableElement<'input, 'arena> {
    type Target = Element<'input, 'arena>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::hash::Hash for HashableElement<'_, '_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}
impl PartialEq for HashableElement<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.id_eq(other)
    }
}
impl Eq for HashableElement<'_, '_> {}

/// An xml element with attributes, (e.g. `<a xlink:href="#" />`)
///
/// [MDN | Element](https://developer.mozilla.org/en-US/docs/Web/API/Element)
impl<'input, 'arena> Element<'input, 'arena> {
    /// Converts the provided node into an element, if the node type matches an element or document
    pub fn new(node: Ref<'input, 'arena>) -> Option<Self> {
        if !matches!(node.node_type(), node::Type::Element | node::Type::Document) {
            return None;
        }
        cfg_if! {
            if #[cfg(feature = "selectors")] {
                Some(Self (node))
            } else {
                Some(Self ( node ))
            }
        }
    }

    /// For a namespaced prefix, finds the alias for the prefix by searching for
    /// the closest matching `xmlns` attribute.
    ///
    /// # Panics
    /// If the given prefix is aliased
    pub fn find_alias(&self, prefix: &Prefix<'input>) -> Option<Atom<'input>> {
        assert!(
            !matches!(prefix, Prefix::Aliased { .. }),
            "Attempted to find alias of already aliased prefix"
        );

        let uri = prefix.ns().uri();
        let mut container = Some(self.clone());
        let mut matching_prefix = None;
        while let Some(inner) = container {
            container = inner.parent_element();

            for attr in inner.attributes() {
                match &*attr {
                    Attr::XMLNS(ns) => {
                        if ns == uri {
                            return None;
                        }
                    }
                    Attr::Unparsed {
                        attr_id:
                            AttrId::Unknown(QualName {
                                prefix: Prefix::XMLNS,
                                local,
                            }),
                        value,
                    } if matching_prefix.is_none() => {
                        if value == uri {
                            matching_prefix = Some(local.clone());
                        }
                    }
                    _ => (),
                }
            }
        }
        matching_prefix
    }

    /// For a given prefix name, finds the namespace URI that the given prefix belongs to be searching for
    /// the closest matching `xmlns` attribute.
    pub fn find_xmlns(&self, prefix: Option<&str>) -> Atom<'input> {
        let mut container = Some(self.clone());
        while let Some(inner) = container {
            container = inner.parent_element();

            for attr in inner.attributes() {
                match &*attr {
                    Attr::XMLNS(ns) => {
                        if prefix.is_none() {
                            return ns.clone();
                        }
                    }
                    Attr::Unparsed {
                        attr_id:
                            AttrId::Unknown(QualName {
                                prefix: Prefix::XMLNS,
                                local,
                            }),
                        value,
                    } => {
                        if let Some(prefix) = prefix {
                            if prefix == local.as_str() {
                                return value.clone();
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
        return NS::SVG.uri().clone();
    }

    /// Parsed a qualified string to an attribute id, namespacing the prefix
    /// based on the closest matching `xmlns` attribute
    ///
    /// # Panics
    ///
    /// If the attribute name is invalid
    pub fn parse_attr_id(&self, qual_name: &str) -> AttrId<'input> {
        let mut parts = qual_name.split(':');
        let prefix_or_local = parts
            .next()
            .expect("Attempted to parse name from empty string");
        let maybe_local = parts.next();
        assert_eq!(
            parts.next(),
            None,
            "Attempted to parse name with multiple `:` characters"
        );

        let prefix = maybe_local.map(|_| prefix_or_local);
        let local = maybe_local.unwrap_or(prefix_or_local);
        let ns = self.find_xmlns(prefix);
        let prefix = Prefix::new(ns, prefix.map(Into::into).map(Atom::into_owned));
        self.qual_name()
            .parse_attr_id(&prefix, Atom::from(local).into_owned())
    }

    /// Returns this element as [Document], even if it's not a document node.
    ///
    /// Only use this as a shortcut to constructors such as `create_element`; other methods may
    /// end up being invalid.
    ///
    /// For other cases, try `element.document()?.as_document()`
    pub fn as_document(&self) -> Document<'input, 'arena> {
        Document(self.clone())
    }

    /// Creates an element from an element's parent type.
    pub fn from_parent(node: Ref<'input, 'arena>) -> Option<Self> {
        Self::new(node)
    }

    /// Returns the element's name as a qualified name.
    pub fn qual_name(&self) -> &ElementId<'input> {
        self.data().name
    }

    /// Returns the local part of the element's qualified name.
    ///
    /// [MDN | localName](https://developer.mozilla.org/en-US/docs/Web/API/Element/localName)
    pub fn local_name(&self) -> &Atom<'input> {
        self.qual_name().local_name()
    }

    /// Returns the namespace prefix of the element's qualified name.
    ///
    /// [MDN | prefix](https://developer.mozilla.org/en-US/docs/Web/API/Element/prefix)
    pub fn prefix(&self) -> &Prefix<'input> {
        self.qual_name().prefix()
    }

    /// Returns the element's tag-name (i.e. it's qualified name) in uppercase.
    ///
    /// [MDN | tagName](https://developer.mozilla.org/en-US/docs/Web/API/Element/tagName)
    pub fn tag_name(&self) -> String {
        self.local_name().to_string().to_uppercase()
    }

    /// Returns the value of an attribute of the element specified by it's qualified name.
    ///
    /// [MDN | getAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttribute)
    pub fn get_attribute<'a>(&'a self, name: &AttrId) -> Option<cell::Ref<'a, Attr<'input>>> {
        self.get_attribute_node(name)
    }

    /// Returns the value of an attribute of the element specified by a local name, only if that
    /// attribute also has no prefix.
    ///
    /// If the name is known, you may prefer to use [`Element::get_attribute`]
    pub fn get_attribute_local<'a>(
        &'a self,
        local_name: &Atom,
    ) -> Option<cell::Ref<'a, Attr<'input>>> {
        self.get_attribute_node_local(local_name)
    }

    /// Returns the value of an attribute of the element specified by it's local name and
    /// namespace.
    ///
    /// [MDN | getAttributeNS](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNS)
    pub fn get_attribute_ns<'a>(
        &'a self,
        namespace: &NS,
        local_name: &Atom,
    ) -> Option<cell::Ref<'a, Attr<'input>>> {
        self.get_attribute_node_ns(namespace, local_name)
    }

    /// Returns a collection of the attribute names of the element.
    ///
    /// [MDN | getAttributeNames](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNames)
    pub fn get_attribute_names<'a, B>(&'a self) -> B
    where
        B: FromIterator<cell::Ref<'a, AttrId<'input>>>,
    {
        self.attributes()
            .into_iter()
            .map(|attr| cell::Ref::map(attr, |attr: &Attr<'input>| attr.name()))
            .collect()
    }

    /// Returns the attribute specified by it's qualified name.
    ///
    /// [MDN | getAttributeNode](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNode)
    fn get_attribute_node<'a>(&'a self, attr_name: &AttrId) -> Option<cell::Ref<'a, Attr<'input>>> {
        self.attributes().get_named_item(attr_name)
    }

    fn get_attribute_node_local<'a>(
        &'a self,
        local_name: &Atom,
    ) -> Option<cell::Ref<'a, Attr<'input>>> {
        self.attributes().get_named_item_local(local_name)
    }

    /// See [`Attributes::get_attribute_node`]
    pub fn get_attribute_node_mut<'a>(
        &'a self,
        attr_name: &AttrId,
    ) -> Option<RefMut<'a, Attr<'input>>> {
        self.attributes().get_named_item_mut(attr_name)
    }

    fn get_attribute_node_ns<'a>(
        &'a self,
        namespace: &NS,
        local_name: &Atom,
    ) -> Option<cell::Ref<'a, Attr<'input>>> {
        self.attributes().get_named_item_ns(namespace, local_name)
    }

    /// Returns whether the element has the specified attribute or not.
    ///
    /// [MDN | hasAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/hasAttribute)
    pub fn has_attribute(&self, name: &AttrId) -> bool {
        self.get_attribute_node(name).is_some()
    }

    /// Returns whether the element has any attributes or not.
    ///
    /// [MDN | hasAttributes](https://developer.mozilla.org/en-US/docs/Web/API/Element/hasAttributes)
    pub fn has_attributes(&'arena self) -> bool {
        !self.attributes().is_empty()
    }

    /// Returns whether the element is the root of the document.
    pub fn is_root(&self) -> bool {
        let Some(parent) = self.parent_node() else {
            return true;
        };
        parent.node_type() == node::Type::Document
    }

    /// Inserts the node before the first child of the element.
    ///
    /// [MDN | prepend](https://developer.mozilla.org/en-US/docs/Web/API/Element/prepend)
    pub fn prepend(&self, node: Ref<'input, 'arena>) {
        if let Some(first_node) = self.first_child.get() {
            first_node.previous_sibling.set(Some(node));
            node.next_sibling.set(Some(first_node));
            self.first_child.set(Some(node));
        } else {
            debug_assert!(self.last_child.get().is_none());
            self.first_child.set(Some(node));
            self.last_child.set(Some(node));
        }
        node.parent.set(Some(self));
    }

    /// Removes the attribute with the specified name from the element.
    ///
    /// [MDN | removeAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/removeAttribute)
    pub fn remove_attribute(&self, attr_name: &AttrId) -> Option<Attr<'input>> {
        let attrs = self.attributes();
        attrs.remove_named_item(attr_name)
    }

    /// Replaces all the children in this element with a new list of children.
    ///
    /// [MDN | replaceChildren](https://developer.mozilla.org/en-US/docs/Web/API/Element/replaceChildren)
    pub fn replace_children(&self, children: impl std::iter::Iterator<Item = Ref<'input, 'arena>>) {
        let mut children = children.peekable();
        let first = children.peek().copied();
        self.first_child.set(first);
        first.inspect(|first| {
            first.previous_sibling.set(None);
        });
        let mut last = first;
        for (a, b) in children.tuple_windows() {
            // NOTE: This only runs if children >= 2
            a.parent.set(Some(self));
            a.next_sibling.set(Some(b));
            b.previous_sibling.set(Some(a));
            last = Some(b);
        }
        last.inspect(|last| {
            last.parent.set(Some(self));
            last.next_sibling.set(None);
        });
        self.last_child.set(last);
    }

    /// Replaces this element in the children list of it's parent with another.
    ///
    /// [MDN | replaceWith](https://developer.mozilla.org/en-US/docs/Web/API/Element/replaceWith)
    fn replace_with(&self, other: Ref<'input, 'arena>) {
        self.after(other);
        self.remove();
    }

    /// Sets the value of the specified attribute on the specified element.
    ///
    /// [MDN | setAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/setAttribute)
    pub fn set_attribute<'a>(&'a self, attr: Attr<'input>) {
        let attrs = self.attributes();
        attrs.set_named_item(attr);
    }

    #[must_use = "The original element is defunct. The returned element should be used instead"]
    /// Sets the local name of the element to a new one. Returns the new element.
    ///
    /// Note that this is usually done by replacing the element with a clone of itself, so
    /// references to the old element will be detached.
    ///
    /// # Panics
    /// If the element wraps a non element node, (e.g. by using `Node::element` instead of `Node::find_element`)
    pub fn set_local_name(
        &self,
        mut new_name: ElementId<'input>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Self {
        let NodeData::Element { attrs, .. } = &self.node_data else {
            panic!("expected an element!");
        };
        let prefix = new_name.prefix();
        let alias = self.find_alias(prefix);
        if alias != new_name.prefix().value() {
            new_name = ElementId::Aliased {
                prefix: Prefix::Aliased {
                    prefix: Box::new(prefix.clone()),
                    alias,
                },
                element_id: Box::new(new_name),
            }
        }
        let replacement = allocator.alloc(NodeData::Element {
            name: new_name,
            attrs: attrs.clone(),
            #[cfg(feature = "selectors")]
            selector_flags: Cell::new(None),
        });
        self.replace_with(replacement);
        Self(&*replacement)
    }

    /// Returns the element immediately following this one in it's parent's child list.
    ///
    /// [MDN | nextElementSibling](https://developer.mozilla.org/en-US/docs/Web/API/Element/nextElementSibling)
    pub fn next_element_sibling(&self) -> Option<Self> {
        let mut saw_self = false;
        for sibling in Element::parent_element(self)?.children() {
            if saw_self {
                return Some(sibling);
            } else if sibling.id_eq(self) {
                saw_self = true;
            }
        }
        None
    }

    /// Returns the element immediately prior to this one in it's parent's child list.
    ///
    /// [MDN | previousElementSibling](https://developer.mozilla.org/en-US/docs/Web/API/Element/previousElementSibling)
    pub fn previous_element_sibling(&self) -> Option<Self> {
        let mut previous = None;
        for sibling in Element::parent_element(self)?.children() {
            if sibling.id_eq(self) {
                return previous;
            }
            previous = Some(sibling);
        }
        None
    }

    /// Inserts a node in the children list of the [Element]'s parent, just after this [Element]
    ///
    /// [MDN | after](https://developer.mozilla.org/en-US/docs/Web/API/Element/after)
    pub fn after(&self, node: Ref<'input, 'arena>) {
        let Some(parent) = self.parent_node() else {
            return;
        };
        parent.insert_after(node, self);
    }

    /// Inserts a node after the last child of the element.
    ///
    /// [MDN | append](https://developer.mozilla.org/en-US/docs/Web/API/Element/append)
    pub fn append(&self, node: Ref<'input, 'arena>) {
        node.remove();
        if let Some(last_node) = self.last_child.get() {
            last_node.next_sibling.set(Some(node));
            node.previous_sibling.set(Some(last_node));
            self.last_child.set(Some(node));
        } else {
            debug_assert!(self.first_child.get().is_none());
            self.first_child.set(Some(node));
            self.last_child.set(Some(node));
        }
        node.parent.set(Some(self));
    }

    /// Inserts a node in the children list of the [Element]'s parent, just before this [Element]
    ///
    /// [MDN | before](https://developer.mozilla.org/en-US/docs/Web/API/Element/before)
    pub fn before(&'arena self, node: Ref<'input, 'arena>) -> Option<()> {
        let parent = self.parent_node()?;
        node.remove();
        node.set_parent_node(&parent);
        parent.insert_before(node, self);
        Some(())
    }

    /// Returns a collection of the attributes assigned to the element.
    ///
    /// [MDN | attributes](https://developer.mozilla.org/en-US/docs/Web/API/Element/attributes)
    pub fn attributes<'a>(&'a self) -> Attributes<'a, 'input> {
        Attributes(self.data().attrs)
    }

    /// Replaces the element's collection of attributes with a new collection.
    pub fn set_attributes(&self, new_attrs: &Attributes<'_, 'input>) {
        let attrs = self.data().attrs;
        attrs.replace(new_attrs.0.take());
    }

    /// Returns the element's parent element.
    ///
    /// [MDN | parentElement](https://developer.mozilla.org/en-US/docs/Web/API/Node/parentElement)
    pub fn parent_element(&self) -> Option<Self> {
        self.parent_node()
    }

    /// Returns the number of child elements of this element.
    ///
    /// If you're checking whether the element is empty of any child elements, consider using
    /// [`Element::has_child_elements`] instead.
    ///
    /// [MDN | childElementCount](https://developer.mozilla.org/en-US/docs/Web/API/Element/childElementCount)
    pub fn child_element_count(&self) -> usize {
        self.children().len()
    }

    /// Returns a collection of the child elements of this element.
    ///
    /// [MDN | children](https://developer.mozilla.org/en-US/docs/Web/API/Element/children)
    pub fn children(&self) -> Vec<Self> {
        self.child_nodes_iter()
            .filter(|n| is_element!(n))
            .filter_map(Self::new)
            .collect()
    }

    /// Returns an iterator that covers each of the child elements of this element.
    #[deprecated(note = "use child_elements_iter to avoid allocation")]
    pub fn children_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        self.children().into_iter()
    }

    /// Returns a [`ClassList`] for manipulating the tokens of a class attribute.
    pub fn class_list<'a>(&'a self) -> ClassList<'a, 'input> {
        ClassList {
            attrs: self.attributes(),
            class_index_memo: Cell::new(0),
        }
    }

    /// Returns whether a class (e.g. `.my-class` or `my-class`) is in the class attribute
    pub fn has_class(&self, token: &str) -> bool {
        let token = token.trim_start_matches('.');
        self.class_list().contains(token)
    }

    /// Traverses the element and it's parents until it finds an element that matches the specified
    /// local-name
    ///
    /// Enable the "selectors" feature if you need to use a css string.
    pub fn closest(&self, name: &ElementId) -> Option<Self> {
        let parent = Element::parent_element(self)?;
        if parent.node_type() == node::Type::Document {
            return None;
        }
        if parent.qual_name() == name {
            Some(parent)
        } else {
            parent.closest(name)
        }
    }

    /// Traverses the element and it's parents until it finds the document node that contains the
    /// element, returning the document as an Element.
    pub fn document(&self) -> Option<Self> {
        let Some(parent) = self.parent_node() else {
            return Some(self.clone());
        };
        match self.node_data {
            NodeData::Element { .. } => parent.document(),
            NodeData::Document | NodeData::Root => Some(parent),
            _ => None,
        }
    }

    /// Returns whether any of the child nodes of this element are elements
    pub fn has_child_elements(&self) -> bool {
        Element::first_element_child(self).is_some()
    }

    /// Returns the element's first child element.
    ///
    /// [MDN | firstElementChild](https://developer.mozilla.org/en-US/docs/Web/API/Element/firstElementChild)
    pub fn first_element_child(&self) -> Option<Self> {
        self.children().into_iter().next()
    }

    /// Returns an node list containing all the child elements of this element
    pub fn child_elements_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        self.child_nodes_iter().filter_map(Self::new)
    }

    /// Replaces the element in the DOM with each of it's child nodes, removing the element in the
    /// process.
    pub fn flatten(&self) {
        let parent = self.parent.take();
        let mut current = self.first_child.get();
        while let Some(current_child) = current {
            current_child.parent.set(parent);
            current = current_child.next_sibling.get();
        }

        let previous_sibling = self.previous_sibling.take();
        let next_sibling = self.next_sibling.take();
        let first_child = self.first_child.take();
        let last_child = self.last_child.take();

        if let Some(first_child) = first_child {
            if let Some(previous_sibling) = previous_sibling {
                previous_sibling.next_sibling.set(Some(first_child));
                first_child.previous_sibling.set(Some(previous_sibling));
            } else if let Some(parent) = parent {
                parent.first_child.set(Some(first_child));
            }
        } else if let Some(previous_sibling) = previous_sibling {
            previous_sibling.next_sibling.set(next_sibling);
            next_sibling.inspect(|n| n.previous_sibling.set(Some(previous_sibling)));
        } else if let Some(parent) = parent {
            parent.first_child.set(next_sibling);
        }
        if let Some(last_child) = last_child {
            if let Some(next_sibling) = next_sibling {
                last_child.next_sibling.set(Some(next_sibling));
                next_sibling.previous_sibling.set(Some(last_child));
            } else if let Some(parent) = parent {
                parent.last_child.set(Some(last_child));
            }
        } else if let Some(next_sibling) = next_sibling {
            next_sibling.previous_sibling.set(previous_sibling);
        } else if let Some(parent) = parent {
            parent.last_child.set(previous_sibling);
        }
    }

    /// Returns the element's last child element.
    ///
    /// [MDN | lastElementChild](https://developer.mozilla.org/en-US/docs/Web/API/Element/lastElementChild)
    pub fn last_element_child(&self) -> Option<Self> {
        self.children().into_iter().next_back()
    }

    /// From a node, do a breadth-first search for the first element contained within it.
    pub fn find_element(node: Ref<'input, 'arena>) -> Option<Self> {
        let mut queue = VecDeque::new();
        queue.push_back(node);

        while let Some(current) = queue.pop_front() {
            let maybe_element = current.element();
            if maybe_element.as_ref().is_some_and(|n| is_element!(n)) {
                return maybe_element;
            }

            for child in current.child_nodes_iter() {
                queue.push_back(child);
            }
        }
        None
    }

    /// Reorder the children of the element based on the given callback.
    pub fn sort_child_elements<F>(&self, mut f: F)
    where
        F: FnMut(Self, Self) -> std::cmp::Ordering,
    {
        let mut children: Vec<_> = self.child_nodes_iter().collect();
        children.sort_by(|a, b| {
            let Some(a) = Element::new(a) else {
                return std::cmp::Ordering::Less;
            };
            let Some(b) = Element::new(b) else {
                return std::cmp::Ordering::Greater;
            };
            f(a, b)
        });

        self.first_child.set(children.first().copied());
        self.last_child.set(children.last().copied());
        for i in 0..children.len() {
            let child = children[i];
            if i > 0 {
                child.previous_sibling.set(children.get(i - 1).copied());
            }
            child.next_sibling.set(children.get(i + 1).copied());
        }
    }

    /// Returns an iterator over the element and it's descendants
    pub fn breadth_first(&self) -> Iterator<'input, 'arena> {
        Iterator::new(self)
    }

    #[cfg(feature = "selectors")]
    /// # Errors
    /// If the selector is invalid
    pub fn select<'a>(
        &'a self,
        selector: &'a str,
    ) -> Result<
        crate::selectors::Select<'input, 'arena>,
        cssparser::ParseError<'a, selectors::parser::SelectorParseErrorKind<'a>>,
    > {
        crate::selectors::Select::new(self, selector)
    }

    #[cfg(feature = "selectors")]
    #[allow(clippy::type_complexity)]
    /// Selects an element with the given selector.
    pub fn select_with_selector(
        &self,
        selector: crate::selectors::Selector,
    ) -> crate::selectors::Select<'input, 'arena> {
        crate::selectors::Select::new_with_selector(self, selector)
    }

    #[cfg(feature = "selectors")]
    /// Sets the selector flags of an element
    pub fn set_selector_flags(&self, flags: selectors::matching::ElementSelectorFlags) {
        let NodeData::Element {
            ref selector_flags, ..
        } = self.node_data
        else {
            return;
        };
        selector_flags.set(Some(flags));
    }

    fn data<'a>(&'a self) -> ElementData<'a, 'input> {
        if let NodeData::Element {
            ref name,
            ref attrs,
            ..
        } = self.node_data
        {
            ElementData { name, attrs }
        } else {
            unreachable!("Element contains non-element data. This is a bug!")
        }
    }
}

impl PartialEq for Element<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.id_eq(other)
    }
}

/// A reference to an element's data
pub struct ElementData<'a, 'input> {
    name: &'a ElementId<'input>,
    attrs: &'a RefCell<Vec<Attr<'input>>>,
}

impl<'input, 'arena> Deref for Element<'input, 'arena> {
    type Target = Ref<'input, 'arena>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Element<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !is_element!(self) {
            return self.0.fmt(f);
        }
        let name = self.qual_name();
        let attributes = self.attributes();
        let child_node_count = self.child_node_count();
        let text = match child_node_count {
            1 => self
                .text_content()
                .map(|s| s.trim().to_string())
                .unwrap_or_default(),
            _ => String::new(),
        };
        f.debug_struct("Element")
            .field("name", name)
            .field("attributes", &attributes)
            .field("text", &text)
            .field("child_count", &child_node_count)
            .finish()
    }
}
