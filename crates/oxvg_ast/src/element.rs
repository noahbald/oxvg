//! XML element traits.
use std::{
    cell::{Ref, RefMut},
    collections::VecDeque,
    fmt::Debug,
    marker::PhantomData,
};

use crate::{
    atom::Atom,
    attribute::{Attr, Attributes},
    class_list::ClassList,
    document::Document,
    name::Name,
    node::{self, Node, Type},
};

/// An xml element with attributes, (e.g. `<a xlink:href="#" />`)
///
/// [MDN | Element](https://developer.mozilla.org/en-US/docs/Web/API/Element)
pub trait Element<'arena>: Node<'arena> + Debug + std::hash::Hash + Eq + PartialEq {
    /// The type representing the tag or attribute name of the element
    type Name: Name;
    /// The type representing a singular attribute of the element's list of attributes
    type Attr: Attr<Name = Self::Name, Atom = <Self as Node<'arena>>::Atom>;
    /// The type representing a list of attributes in an element
    type Attributes<'a>: Attributes<'a, Attribute = Self::Attr>;
    /// `Self` with a lifetime argument
    type Lifetimed<'a>: Element<'a>;

    /// Converts the provided node into an element, if the node type matches an element or document
    fn new(node: Self::Child) -> Option<Self>;

    /// Returns this element as [Document], even if it's not a document node.
    ///
    /// Only use this as a shortcut to constructors such as `create_element`; other methods may
    /// end up being invalid.
    ///
    /// For other cases, try `element.document()?.as_document()`
    fn as_document(&self) -> impl Document<'arena, Root = Self>;

    /// Creates an element from an element's parent type.
    fn from_parent(node: Self::ParentChild) -> Option<Self>;

    /// Returns a collection of the attributes assigned to the element.
    ///
    /// [MDN | attributes](https://developer.mozilla.org/en-US/docs/Web/API/Element/attributes)
    fn attributes(&self) -> Self::Attributes<'_>;

    /// Replaces the element's collection of attributes with a new collection.
    fn set_attributes(&self, new_attrs: Self::Attributes<'_>);

    /// Returns the number of child elements of this element.
    ///
    /// If you're checking whether the element is empty of any child elements, consider using
    /// [`Element::has_child_elements`] instead.
    ///
    /// [MDN | childElementCount](https://developer.mozilla.org/en-US/docs/Web/API/Element/childElementCount)
    fn child_element_count(&self) -> usize {
        self.children().len()
    }

    /// Returns a collection of the child elements of this element.
    ///
    /// If you're calling this only to iterate over the children, consider using
    /// [`Element::children_iter`] instead.
    ///
    /// [MDN | children](https://developer.mozilla.org/en-US/docs/Web/API/Element/children)
    fn children(&self) -> Vec<Self> {
        self.child_nodes_iter()
            .filter(|n| matches!(n.node_type(), Type::Element))
            .filter_map(|n| Self::new(n))
            .collect()
    }

    /// Returns an iterator that covers each of the child elements of this element.
    #[deprecated]
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        self.children().into_iter()
    }

    /// Returns a [`ClassList`] for manipulating the tokens of a class attribute.
    fn class_list(
        &self,
    ) -> impl ClassList<Attribute = <Self::Attributes<'_> as Attributes>::Attribute>;

    /// Returns whether a class (e.g. `.my-class` or `my-class`) is in the class attribute
    fn has_class(&self, token: &Self::Atom) -> bool {
        let token = token.trim_start_matches('.');
        self.class_list().contains(&token.into())
    }

    /// Traverses the element and it's parents until it finds an element that matches the specified
    /// local-name
    ///
    /// Enable the "selectors" feature if you need to use a css string.
    fn closest_local(&self, name: &<Self::Name as Name>::LocalName) -> Option<Self> {
        let parent = Element::parent_element(self)?;
        if parent.node_type() == node::Type::Document {
            return None;
        }
        if parent.local_name() == name {
            Some(parent)
        } else {
            parent.closest_local(name)
        }
    }

    /// Traverses the element and it's parents until it finds the document node that contains the
    /// element, returning the document as an Element.
    fn document(&self) -> Option<Self>;

    /// Returns whether any of the child nodes of this element are elements
    fn has_child_elements(&self) -> bool {
        Element::first_element_child(self).is_some()
    }

    /// Returns the element's first child element.
    ///
    /// [MDN | firstElementChild](https://developer.mozilla.org/en-US/docs/Web/API/Element/firstElementChild)
    fn first_element_child(&self) -> Option<Self> {
        self.children().into_iter().next()
    }

    /// Returns an node list containing all the child elements of this element
    fn child_elements_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        self.child_nodes_iter().filter_map(Self::new)
    }

    /// Reorder the children of the element based on the given callback.
    fn sort_child_elements<F>(&self, f: F)
    where
        F: FnMut(Self, Self) -> std::cmp::Ordering;

    /// Replaces the element in the DOM with each of it's child nodes, removing the element in the
    /// process.
    fn flatten(&self);

    /// Returns the element's last child element.
    ///
    /// [MDN | lastElementChild](https://developer.mozilla.org/en-US/docs/Web/API/Element/lastElementChild)
    fn last_element_child(&self) -> Option<Self> {
        self.children().into_iter().next_back()
    }

    /// Returns the element's name as a qualified name.
    fn qual_name(&self) -> &Self::Name;

    /// Returns the local part of the element's qualified name.
    ///
    /// [MDN | localName](https://developer.mozilla.org/en-US/docs/Web/API/Element/localName)
    fn local_name(&self) -> &<Self::Name as Name>::LocalName {
        self.qual_name().local_name()
    }

    /// Sets the local name of the element to a new one.
    ///
    /// Note that this is usually done by replacing the element with a clone of itself, so
    /// references to the old element will be detached.
    fn set_local_name(&self, name: <Self::Name as Name>::LocalName, arena: &Self::Arena);

    /// Returns the element immediately following this one in it's parent's child list.
    ///
    /// [MDN | nextElementSibling](https://developer.mozilla.org/en-US/docs/Web/API/Element/nextElementSibling)
    fn next_element_sibling(&self) -> Option<Self> {
        let mut saw_self = false;
        for sibling in Element::parent_element(self)?.children() {
            if saw_self {
                return Some(sibling);
            } else if sibling.ptr_eq(self) {
                saw_self = true;
            }
        }
        None
    }

    /// Returns the namespace prefix of the element's qualified name.
    ///
    /// [MDN | prefix](https://developer.mozilla.org/en-US/docs/Web/API/Element/prefix)
    fn prefix(&self) -> &Option<<Self::Name as Name>::Prefix> {
        self.qual_name().prefix()
    }

    /// Returns the element immediately prior to this one in it's parent's child list.
    ///
    /// [MDN | previousElementSibling](https://developer.mozilla.org/en-US/docs/Web/API/Element/previousElementSibling)
    fn previous_element_sibling(&self) -> Option<Self> {
        let mut previous = None;
        for sibling in Element::parent_element(self)?.children() {
            if sibling.ptr_eq(self) {
                return previous;
            }
            previous = Some(sibling);
        }
        None
    }

    /// Returns the element's tag-name (i.e. it's qualified name) in uppercase.
    ///
    /// [MDN | tagName](https://developer.mozilla.org/en-US/docs/Web/API/Element/tagName)
    fn tag_name(&self) -> Self::Atom {
        let local_name = self.local_name();
        match self.prefix() {
            Some(prefix) => format!("{prefix}:{local_name}").to_uppercase().into(),
            None => local_name.as_str().to_uppercase().into(),
        }
    }

    /// Inserts a node in the children list of the [Element]'s parent, just after this [Element]
    ///
    /// [MDN | after](https://developer.mozilla.org/en-US/docs/Web/API/Element/after)
    fn after(&self, node: <Self as Node<'arena>>::ParentChild) {
        let Some(mut parent) = self.parent_node() else {
            return;
        };
        node.remove();
        node.set_parent_node(&parent);
        parent.insert_after(node, &self.as_parent_child());
    }

    /// Inserts a node after the last child of the element.
    ///
    /// [MDN | append](https://developer.mozilla.org/en-US/docs/Web/API/Element/append)
    fn append(&self, node: Self::Child);

    /// Inserts a node in the children list of the [Element]'s parent, just before this [Element]
    ///
    /// [MDN | before](https://developer.mozilla.org/en-US/docs/Web/API/Element/before)
    fn before(&self, node: <Self as Node<'arena>>::ParentChild) -> Option<()> {
        let mut parent = self.parent_node()?;
        node.remove();
        node.set_parent_node(&parent);
        parent.insert_before(node, &self.as_parent_child());
        Some(())
    }

    /// From a node, do a breadth-first search for the first element contained within it.
    fn find_element(node: <Self as Node<'arena>>::ParentChild) -> Option<Self>;

    /// Returns the value of an attribute of the element specified by it's qualified name.
    ///
    /// [MDN | getAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttribute)
    fn get_attribute<'a>(
        &'a self,
        name: &<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> Option<Ref<'a, Self::Atom>>
    where
        'arena: 'a,
    {
        self.get_attribute_node(name)
            .map(|a| Ref::map(a, |a| a.value()))
    }

    /// Returns the value of an attribute of the element specified by a local name, only if that
    /// attribute also has no prefix
    fn get_attribute_local<'a>(
        &'a self,
        name: &<<Self::Attr as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, Self::Atom>>
    where
        'arena: 'a,
    {
        self.get_attribute_node_local(name)
            .map(|a| Ref::map(a, |a| a.value()))
    }

    /// Returns the value of an attribute of the element specified by it's local name and
    /// namespace.
    ///
    /// [MDN | getAttributeNS](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNS)
    fn get_attribute_ns<'a>(
        &'a self,
        namespace: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, Self::Atom>>
    where
        'arena: 'a,
    {
        self.get_attribute_node_ns(namespace, name)
            .map(|a| Ref::map(a, |a| a.value()))
    }

    /// Returns a collection of the attribute names of the element.
    ///
    /// [MDN | getAttributeNames](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNames)
    fn get_attribute_names<'a, B>(&'a self) -> B
    where
        B: FromIterator<
            Ref<'a, <<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name>,
        >,
        'arena: 'a,
    {
        self.attributes()
            .into_iter()
            .map(|attr| Ref::map(attr, |attr| attr.name()))
            .collect()
    }

    /// Returns the attribute specified by it's qualified name.
    ///
    /// [MDN | getAttributeNode](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNode)
    fn get_attribute_node<'a>(
        &'a self,
        attr_name: &<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> Option<Ref<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item(attr_name)
    }

    /// See [`Attributes::get_attribute_node`]
    fn get_attribute_node_mut<'a>(
        &'a self,
        attr_name: &<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> Option<RefMut<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item_mut(attr_name)
    }

    /// Returns the attribute of the element specified by a local name, only if that
    /// attribute also has no prefix
    fn get_attribute_node_local<'a>(
        &'a self,
        attr_name: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item_local(attr_name)
    }

    /// Returns the mutable attribute of the element specified by a local name, only if that
    /// attribute also has no prefix
    fn get_attribute_node_local_mut<'a>(
        &'a self,
        attr_name: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<RefMut<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item_local_mut(attr_name)
    }

    /// Returns the attribute specified by it's localname and namespace
    ///
    /// [MDN getAttributeNodeNS](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNodeNS)
    fn get_attribute_node_ns<'a>(
        &'a self,
        namespace: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item_ns(namespace, name)
    }

    /// Returns whether the element has the specified attribute or not.
    ///
    /// [MDN | hasAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/hasAttribute)
    fn has_attribute<'a, N>(&'a self, name: &N) -> bool
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        N: Name,
        'arena: 'a,
    {
        self.get_attribute_node(name).is_some()
    }

    /// Returns whether the element has the specified attribute or not by a local name, only if that
    /// attribute also has no prefix
    fn has_attribute_local<'a, N>(&'a self, name: &N) -> bool
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
        'arena: 'a,
    {
        self.get_attribute_node_local(name).is_some()
    }

    /// Returns whether the element has any attributes or not.
    ///
    /// [MDN | hasAttributes](https://developer.mozilla.org/en-US/docs/Web/API/Element/hasAttributes)
    fn has_attributes<'a, N>(&'a self) -> bool
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        N: Name,
    {
        !self.attributes().is_empty()
    }

    /// Returns whether the element is the root of the document.
    fn is_root(&self) -> bool {
        let Some(parent) = self.parent_node() else {
            return true;
        };
        parent.node_type() == node::Type::Document
    }

    /// Inserts the node before the first child of the element.
    ///
    /// [MDN | prepend](https://developer.mozilla.org/en-US/docs/Web/API/Element/prepend)
    fn prepend(&self, other: Self::ParentChild) {
        let Some(mut parent) = self.parent_node() else {
            return;
        };
        other.remove();
        other.set_parent_node(&parent);
        parent.insert_before(other, &self.as_parent_child());
    }

    /// Removes the attribute with the specified name from the element.
    ///
    /// [MDN | removeAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/removeAttribute)
    fn remove_attribute<'a, N>(&'a self, attr_name: &N)
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        N: Name,
    {
        let attrs = self.attributes();
        attrs.remove_named_item(attr_name);
    }

    /// Removes the attribute with the specified local name from the element, only if that
    /// attribute also has no prefix
    fn remove_attribute_local<'a, N>(
        &'a self,
        attr_name: &N,
    ) -> Option<<Self::Attributes<'a> as Attributes<'a>>::Attribute>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
    {
        let attrs = self.attributes();
        attrs.remove_named_item_local(attr_name)
    }

    /// Replaces all the children in this element with a new list of children.
    ///
    /// [MDN | replaceChildren](https://developer.mozilla.org/en-US/docs/Web/API/Element/replaceChildren)
    fn replace_children(&self, children: Vec<Self::Child>);

    /// Replaces this element in the children list of it's parent with another.
    ///
    /// [MDN | replaceWith](https://developer.mozilla.org/en-US/docs/Web/API/Element/replaceWith)
    fn replace_with(&self, other: Self::ParentChild) {
        self.after(other);
        self.remove();
    }

    /// Sets the value of the specified attribute on the specified element.
    ///
    /// [MDN | setAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/setAttribute)
    fn set_attribute<'a>(
        &'a self,
        attr_name: <<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
        value: <<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Atom,
    ) {
        let attrs = self.attributes();
        let new_attr = <Self::Attributes<'a> as Attributes<'a>>::Attribute::new(attr_name, value);
        attrs.set_named_item(new_attr);
    }

    /// Sets the value of the specified attribute on the element by local name, without any prefix
    fn set_attribute_local(
        &self,
        attr_name: <<Self::Attr as Attr>::Name as Name>::LocalName,
        value: <Self::Attr as Attr>::Atom,
    ) {
        let attrs = self.attributes();
        let qual_name = <Self::Attr as Attr>::Name::new(None, attr_name);
        let new_attr = Self::Attr::new(qual_name, value);
        attrs.set_named_item(new_attr);
    }

    /// Returns the element's parent element.
    ///
    /// [MDN | parentElement](https://developer.mozilla.org/en-US/docs/Web/API/Node/parentElement)
    fn parent_element(&self) -> Option<Self>;

    /// Returns an iterator over the element and it's descendants
    fn breadth_first(&self) -> Iterator<'arena, Self> {
        Iterator::new(self)
    }

    #[cfg(feature = "selectors")]
    /// # Errors
    /// If the selector is invalid
    fn select<'a>(
        &'a self,
        selector: &'a str,
    ) -> Result<
        crate::selectors::Select<'arena, Self>,
        cssparser::ParseError<'a, selectors::parser::SelectorParseErrorKind<'a>>,
    > {
        crate::selectors::Select::new(self, selector)
    }

    #[cfg(feature = "selectors")]
    #[allow(clippy::type_complexity)]
    /// Selects an element with the given selector.
    fn select_with_selector(
        &self,
        selector: crate::selectors::Selector<
            Self::Atom,
            <Self::Name as Name>::Prefix,
            <Self::Name as Name>::LocalName,
            <Self::Name as Name>::Namespace,
        >,
    ) -> crate::selectors::Select<'arena, Self> {
        crate::selectors::Select::new_with_selector(self, selector)
    }

    #[cfg(feature = "selectors")]
    /// Sets the selector flags of an element
    fn set_selector_flags(&self, flags: selectors::matching::ElementSelectorFlags);
}

#[derive(Debug)]
/// An iterator that goes over an element and it's descendants in a breadth-first fashion
pub struct Iterator<'arena, E: crate::element::Element<'arena>> {
    queue: VecDeque<E>,
    marker: PhantomData<&'arena ()>,
}

impl<'arena, E: crate::element::Element<'arena>> Iterator<'arena, E> {
    /// Returns a breadth-first iterator starting at the given element
    pub fn new(element: &E) -> Self {
        let mut queue = VecDeque::new();
        element.child_elements_iter().for_each(|e| {
            queue.push_back(e);
        });

        Self {
            queue,
            marker: PhantomData,
        }
    }
}

impl<'arena, E: crate::element::Element<'arena>> std::iter::Iterator for Iterator<'arena, E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.queue.pop_front()?;
        current.child_elements_iter().for_each(|e| {
            self.queue.push_back(e);
        });
        Some(current)
    }
}
