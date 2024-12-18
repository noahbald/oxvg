use std::fmt::Debug;

use crate::{
    atom::Atom,
    attribute::{Attr, Attributes},
    class_list::ClassList,
    name::Name,
    node::{Node, Type},
};

#[cfg(not(feature = "selectors"))]
pub trait Features {}

#[cfg(feature = "selectors")]
pub trait Features: selectors::Element {}

pub trait Element: Node + Features + Debug {
    type Name: Name;
    type Attributes<'a>: Attributes<
        'a,
        Attribute: Attr<Name = Self::Name, Atom = <Self as Node>::Atom>,
    >;

    /// Converts the provided node into an element, if the node type matches an element or document
    fn new(node: Self::Child) -> Option<Self>;

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
        self.children_iter().collect()
    }

    /// Returns an iterator that covers each of the child elements of this element.
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        self.child_nodes_iter()
            .filter(|n| matches!(n.node_type(), Type::Element))
            .filter_map(|n| Self::new(n))
    }

    fn class_list(
        &self,
    ) -> impl ClassList<Attribute = <Self::Attributes<'_> as Attributes>::Attribute>;

    /// Returns whether a class (e.g. `.my-class` or `my-class`) is in the class attribute
    fn has_class(&self, token: &Self::Atom) -> bool;

    /// Traverses the element and it's parents until it finds an element that matches the specified
    /// local-name
    ///
    /// Enable the "selectors" feature if you need to use a css string.
    fn closest_local(&self, name: &<Self::Name as Name>::LocalName) -> Option<Self> {
        let parent = Element::parent_element(self)?;
        if &parent.local_name() == name {
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
        self.children_iter().next()
    }

    /// Replaces the element in the DOM with each of it's child nodes, removing the element in the
    /// process.
    fn flatten(&self);

    /// Returns the element's last child element.
    ///
    /// [MDN | lastElementChild](https://developer.mozilla.org/en-US/docs/Web/API/Element/lastElementChild)
    fn last_element_child(&self) -> Option<Self> {
        self.children_iter().next_back()
    }

    /// Returns the element's name as a qualified name.
    fn qual_name(&self) -> Self::Name;

    /// Returns the local part of the element's qualified name.
    ///
    /// [MDN | localName](https://developer.mozilla.org/en-US/docs/Web/API/Element/localName)
    fn local_name(&self) -> <Self::Name as Name>::LocalName {
        self.qual_name().local_name()
    }

    /// Sets the local name of the element to a new one.
    ///
    /// Note that this is usually done by replacing the element with a clone of itself, so
    /// references to the old element will be outdated.
    fn set_local_name(&mut self, name: <Self::Name as Name>::LocalName);

    /// Returns the element immediately following this one in it's parent's child list.
    ///
    /// [MDN | nextElementSibling](https://developer.mozilla.org/en-US/docs/Web/API/Element/nextElementSibling)
    fn next_element_sibling(&self) -> Option<Self> {
        let mut saw_self = false;
        for sibling in Element::parent_element(self)?.children_iter() {
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
    fn prefix(&self) -> Option<<Self::Name as Name>::Prefix> {
        self.qual_name().prefix()
    }

    /// Returns the element immediately prior to this one in it's parent's child list.
    ///
    /// [MDN | previousElementSibling](https://developer.mozilla.org/en-US/docs/Web/API/Element/previousElementSibling)
    fn previous_element_sibling(&self) -> Option<Self> {
        let mut previous = None;
        for sibling in Element::parent_element(self)?.children_iter() {
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
    fn after(&self, node: <Self as Node>::ParentChild) {
        let Some(mut parent) = self.parent_node() else {
            return;
        };
        node.remove();
        node.set_parent_node(&parent);
        parent.insert_after(node, self.as_parent_child());
    }

    /// Inserts a node after the last child of the element.
    ///
    /// [MDN | append](https://developer.mozilla.org/en-US/docs/Web/API/Element/append)
    fn append(&self, node: Self::Child);

    /// Inserts a node in the children list of the [Element]'s parent, just before this [Element]
    ///
    /// [MDN | before](https://developer.mozilla.org/en-US/docs/Web/API/Element/before)
    fn before(&self, node: <Self as Node>::ParentChild) -> Option<()> {
        let mut parent = self.parent_node()?;
        node.remove();
        node.set_parent_node(&parent);
        parent.insert_before(node, self.as_parent_child());
        Some(())
    }

    fn find_element(node: <Self as Node>::ParentChild) -> Option<Self>;

    /// Returns the value of an attribute of the element specified by it's qualified name.
    ///
    /// [MDN | getAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttribute)
    fn get_attribute<'a, N>(&'a self, name: &N) -> Option<Self::Atom>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        N: Name,
    {
        Some(self.get_attribute_node(name)?.value())
    }

    /// Returns the value of an attribute of the element specified by a local name.
    fn get_attribute_local<'a, N>(&'a self, name: &N) -> Option<Self::Atom>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
    {
        Some(self.get_attribute_node_local(name)?.value())
    }

    /// Returns the value of an attribute of the element specified by it's local name and
    /// namespace.
    ///
    /// [MDN | getAttributeNS](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNS)
    fn get_attribute_ns<'a, N, NS>(&'a self, namespace: &NS, name: &N) -> Option<Self::Atom>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<Namespace = NS>>>,
        NS: Atom,
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
    {
        Some(self.get_attribute_node_ns(namespace, name)?.value())
    }

    /// Returns a collection of the attribute names of the element.
    ///
    /// [MDN | getAttributeNames](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNames)
    fn get_attribute_names<'a, N>(&'a self) -> Vec<N>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        N: Name,
    {
        let attrs = self.attributes();
        let mut names = Vec::with_capacity(attrs.len());

        for i in 0..attrs.len() {
            let Some(attr) = attrs.item(i) else { continue };
            names.push(attr.name());
        }
        names
    }

    /// Returns the attribute specified by it's qualified name.
    ///
    /// [MDN | getAttributeNode](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNode)
    fn get_attribute_node<'a, N>(
        &'a self,
        attr_name: &N,
    ) -> Option<<Self::Attributes<'a> as Attributes>::Attribute>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        N: Name,
    {
        self.attributes().get_named_item(attr_name)
    }

    /// Returns the attribute specified by it's local name.
    fn get_attribute_node_local<'a, N>(
        &'a self,
        attr_name: &N,
    ) -> Option<<Self::Attributes<'a> as Attributes>::Attribute>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
    {
        self.attributes().get_named_item_local(attr_name)
    }

    /// Returns the attribute specified by it's localname and namespace
    ///
    /// [MDN getAttributeNodeNS](https://developer.mozilla.org/en-US/docs/Web/API/Element/getAttributeNodeNS)
    fn get_attribute_node_ns<'a, N, NS>(
        &'a self,
        namespace: &NS,
        name: &N,
    ) -> Option<<Self::Attributes<'a> as Attributes>::Attribute>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<Namespace = NS>>>,
        NS: Atom,
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
    {
        self.attributes().get_named_item_ns(namespace, name)
    }

    /// Returns whether the element has the specified attribute or not.
    ///
    /// [MDN | hasAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Element/hasAttribute)
    fn has_attribute<'a, N>(&'a self, name: &N) -> bool
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        N: Name,
    {
        self.get_attribute_node(name).is_some()
    }

    /// Returns whether the element has the specified attribute or not by it's local name.
    fn has_attribute_local<'a, N>(&'a self, name: &N) -> bool
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
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

    /// Inserts the node before the first child of the element.
    ///
    /// [MDN | prepend](https://developer.mozilla.org/en-US/docs/Web/API/Element/prepend)
    fn prepend(&self, other: Self::ParentChild) {
        let Some(mut parent) = self.parent_node() else {
            return;
        };
        other.remove();
        other.set_parent_node(&parent);
        parent.insert_before(other, self.as_parent_child());
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

    /// Removes the attribute with the specified local-name from the element.
    fn remove_attribute_local<'a, N>(&'a self, attr_name: &N)
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        N: Atom,
    {
        let attrs = self.attributes();
        attrs.remove_named_item_local(attr_name);
    }

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
    fn set_attribute<'a, N, V>(&'a self, attr_name: N, value: V)
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name = N>>,
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Atom = V>>,
        N: Name,
        V: Atom,
    {
        let attrs = self.attributes();
        attrs.set_named_item((attr_name, value).into());
    }

    /// Sets the value of the specified attribute on the specified element by local-name.
    fn set_attribute_local<'a, N, V>(&'a self, attr_name: N, value: V)
    where
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Name: Name<LocalName = N>>>,
        Self::Attributes<'a>: Attributes<'a, Attribute: Attr<Atom = V>>,
        N: Atom,
        V: Atom,
    {
        let attrs = self.attributes();
        attrs.set_named_item((attr_name, value).into());
    }

    /// Returns the element's parent element.
    ///
    /// [MDN | parentElement](https://developer.mozilla.org/en-US/docs/Web/API/Node/parentElement)
    fn parent_element(&self) -> Option<Self>;

    #[cfg(feature = "selectors")]
    /// # Errors
    /// If the selector is invalid
    fn select<'a>(
        &'a self,
        selector: &'a str,
    ) -> Result<
        crate::selectors::Select<Self>,
        cssparser::ParseError<'_, selectors::parser::SelectorParseErrorKind<'_>>,
    > {
        crate::selectors::Select::new(self, selector)
    }

    #[cfg(feature = "selectors")]
    /// Creates an iterator which traverses the elements in a depth-first fashion.
    fn depth_first(&self) -> crate::selectors::ElementIterator<Self> {
        crate::selectors::ElementIterator::new(self)
    }
}
