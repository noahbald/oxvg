use std::fmt::Debug;

use crate::{
    atom::Atom,
    attribute::{Attr, Attributes},
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
        Attribute<'a>: Attr<'a, Name = Self::Name, Atom = <Self as Node>::Atom>,
    >;

    fn new(node: impl Node) -> Option<Self>;

    fn attributes(&self) -> Self::Attributes<'_>;

    fn child_element_count(&self) -> usize {
        self.children().len()
    }

    fn children(&self) -> Vec<Self> {
        self.children_iter().collect()
    }

    fn children_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        self.child_nodes_iter()
            .filter(|n| matches!(n.node_type(), Type::Element))
            .filter_map(|n| Self::new(n))
    }

    fn first_element_child(&self) -> Option<Self> {
        self.children_iter().next()
    }

    fn last_element_child(&self) -> Option<Self> {
        self.children_iter().next_back()
    }

    fn local_name(&self) -> <Self::Name as Name>::LocalName;

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

    fn prefix(&self) -> Option<<Self::Name as Name>::Prefix>;

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

    fn tag_name(&self) -> Self::Atom {
        let value: &str = &self.local_name().into();
        value.to_uppercase().into()
    }

    /// Inserts a node in the children list of the [Element]'s parent, just after the [Element]
    fn after(&self, node: <Self as Node>::ParentChild) {
        let Some(mut parent) = self.parent_node() else {
            return;
        };
        parent.insert_after(node, self.as_parent_child());
    }

    fn append(&self, node: Self::Child) {
        self.child_nodes().push(node);
    }

    fn before(&self, node: <Self as Node>::ParentChild) -> Option<()> {
        let mut parent = self.parent_node()?;
        parent.insert_before(node, self.as_parent_child());
        Some(())
    }

    fn get_attribute<'a, N>(&'a self, name: &N) -> Option<Self::Atom>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name: Name<LocalName = N>>>,
        N: Atom,
    {
        Some(self.get_attribute_node(name)?.value())
    }

    fn get_attribute_ns<'a, N, NS>(&'a self, namespace: &NS, name: &N) -> Option<Self::Atom>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name: Name<Namespace = NS>>>,
        NS: Atom,
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name: Name<LocalName = N>>>,
        N: Atom,
    {
        Some(self.get_attribute_node_ns(namespace, name)?.value())
    }

    fn get_attribute_names<'a, N>(&'a self) -> Vec<N>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name = N>>,
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

    fn get_attribute_node<'a, N>(
        &'a self,
        attr_name: &N,
    ) -> Option<<Self::Attributes<'a> as Attributes>::Attribute<'a>>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name: Name<LocalName = N>>>,
        N: Atom,
    {
        self.attributes().get_named_item(attr_name)
    }

    fn get_attribute_node_ns<'a, N, NS>(
        &'a self,
        namespace: &NS,
        name: &N,
    ) -> Option<<Self::Attributes<'a> as Attributes>::Attribute<'a>>
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name: Name<Namespace = NS>>>,
        NS: Atom,
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name: Name<LocalName = N>>>,
        N: Atom,
    {
        self.attributes().get_named_item_ns(namespace, name)
    }

    fn has_attribute<'a, N>(&'a self, name: &N) -> bool
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name: Name<LocalName = N>>>,
        N: Atom,
    {
        self.get_attribute_node(name).is_some()
    }

    fn has_attributes<'a, N>(&'a self, names: &[N]) -> bool
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name = N>>,
        N: Name,
    {
        let attrs = self.attributes();
        for i in 0..attrs.len() {
            let attr = attrs.item(i).expect("i > attrs.len()");
            if names.contains(&attr.name()) {
                return true;
            }
        }
        false
    }

    fn prepend(&self, other: Self::ParentChild) {
        let Some(mut parent) = self.parent_node() else {
            return;
        };
        parent.insert_before(other, self.as_parent_child());
    }

    fn remove(&self);

    fn remove_attribute<'a, N>(&'a self, attr_name: &N)
    where
        Self::Attributes<'a>: Attributes<'a, Attribute<'a>: Attr<'a, Name = N>>,
        N: Name,
    {
        let attrs = self.attributes();
        attrs.remove_named_item(attr_name);
    }

    fn replace_with(&self, other: Self::ParentChild) {
        self.after(other);
        self.remove();
    }

    fn parent_element(&self) -> Option<Self>;
}
