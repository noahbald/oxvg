//! XML types. Has implementations for all the traits in this crate.
use std::{
    borrow::Borrow,
    cell::{self, Cell, RefCell, RefMut},
    collections::VecDeque,
    fmt::Debug,
};

use cfg_if::cfg_if;
use markup5ever::local_name;
use tendril::StrTendril;

use crate::{
    attribute::{Attr, Attributes as _},
    document,
    element::{self, Element as _},
    name::Name,
    node::{self, Node as _},
};

/// An allocator for a node
pub type Arena<'arena> = &'arena typed_arena::Arena<Node<'arena>>;
/// A reference to a node
pub type Ref<'arena> = &'arena Node<'arena>;
/// A settable reference to a node
pub type Link<'arena> = Cell<Option<Ref<'arena>>>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
/// A qualified name used for the names of tags and attributes.
pub struct QualName {
    /// The local name (e.g. the `href` of `xlink:href`) of a qualified name.
    pub local: string_cache::Atom<markup5ever::LocalNameStaticSet>,
    /// The prefix (e.g. `xlink` of `xlink:href`) of a qualified name.
    pub prefix: Option<string_cache::Atom<markup5ever::PrefixStaticSet>>,
    /// The resolved uri of the name
    pub ns: string_cache::Atom<markup5ever::NamespaceStaticSet>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
/// The attribute of an element's attributes.
pub struct Attribute {
    /// The name of an attribute (e.g. `foo` of `foo="bar"`)
    pub name: QualName,
    /// The value of an attribute (e.g. `"bar"` of `foo="bar"`)
    pub value: StrTendril,
}

#[derive(Clone)]
/// The list of attributes of an element.
pub struct Attributes<'arena>(pub &'arena RefCell<Vec<Attribute>>);

/// A whitespace seperated set of tokens of a class attribute's value.
pub struct ClassList<'arena> {
    pub(crate) attrs: Attributes<'arena>,
    pub(crate) class_index_memo: Cell<usize>,
    pub(crate) tokens: Vec<StrTendril>,
}

#[derive(derive_more::Debug, Clone)]
/// The data of a node in an XML document.
pub enum NodeData {
    /// The document.
    Document,
    /// The root of a document, contains the root element, doctype, PIs, etc.
    Root,
    /// An element. (e.g. <a xlink:href="#">hello</a>)
    Element {
        /// The qualified name of the element's tag.
        name: QualName,
        /// The attributes of the element.
        attrs: RefCell<Vec<Attribute>>,
        #[cfg(feature = "selectors")]
        #[debug(skip)]
        /// Flags used for caching whether an element matches a selector
        selector_flags: Cell<Option<selectors::matching::ElementSelectorFlags>>,
    },
    /// A processing instruction. (e.g. <?xml version="1.0"?>)
    PI {
        /// The name of the application to which the instruction is targeted
        target: StrTendril,
        /// Data for the application
        value: RefCell<Option<StrTendril>>,
    },
    /// A comment node. (e.g. `<!-- foo ->`)
    Comment(RefCell<Option<StrTendril>>),
    /// A text node. (e.g. `foo` of `<p>foo</p>`)
    Text(RefCell<Option<StrTendril>>),
}

#[derive(Clone)]
/// An XML node type.
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
    pub node_data: NodeData,
}

#[derive(Clone)]
/// An XML element type.
pub struct Element<'arena> {
    node: Ref<'arena>,
}

/// A reference to an element's data
pub struct ElementData<'a> {
    name: &'a QualName,
    attrs: &'a RefCell<Vec<Attribute>>,
}

#[derive(Clone)]
/// An XML document type with a root element
pub struct Document<'arena>(Element<'arena>);

impl Name for QualName {
    type LocalName = string_cache::Atom<markup5ever::LocalNameStaticSet>;
    type Prefix = string_cache::Atom<markup5ever::PrefixStaticSet>;
    type Namespace = string_cache::Atom<markup5ever::NamespaceStaticSet>;

    fn new(prefix: Option<Self::Prefix>, local: Self::LocalName) -> Self {
        QualName {
            prefix,
            local,
            ns: Self::Namespace::default(),
        }
    }

    fn local_name(&self) -> &Self::LocalName {
        &self.local
    }

    fn prefix(&self) -> &Option<Self::Prefix> {
        &self.prefix
    }

    fn ns(&self) -> &Self::Namespace {
        &self.ns
    }
}

impl std::hash::Hash for QualName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self { prefix, ns, local } = self;

        prefix.hash(state);
        ns.hash(state);
        local.hash(state);
    }
}

impl Attribute {
    #[cfg(feature = "style")]
    /// Returns the attribute as a presentation attribute.
    pub fn presentation<'a, P>(
        prefix: Option<&'a P>,
        local: &'a str,
        value: &'a str,
    ) -> Option<crate::style::PresentationAttr<'a>> {
        if prefix.is_some() {
            return None;
        }
        let id = crate::style::PresentationAttrId::from(local);
        crate::style::PresentationAttr::parse_string(
            id,
            value,
            lightningcss::stylesheet::ParserOptions::default(),
        )
        .ok()
    }
}

impl Attr for Attribute {
    type Atom = StrTendril;
    type Name = QualName;

    fn new(name: Self::Name, value: Self::Atom) -> Self {
        Attribute { name, value }
    }

    fn name(&self) -> &Self::Name {
        &self.name
    }

    fn name_mut(&mut self) -> &mut Self::Name {
        &mut self.name
    }

    fn local_name(&self) -> &<Self::Name as Name>::LocalName {
        &self.name.local
    }

    fn prefix(&self) -> &Option<<Self::Name as Name>::Prefix> {
        &self.name.prefix
    }

    fn value(&self) -> &Self::Atom {
        &self.value
    }

    fn value_mut(&mut self) -> &mut Self::Atom {
        &mut self.value
    }

    fn set_value(&mut self, value: Self::Atom) -> Self::Atom {
        std::mem::replace(&mut self.value, value)
    }

    fn push(&mut self, value: &Self::Atom) {
        self.value.push_tendril(value);
    }

    fn sub_value(&self, offset: u32, length: u32) -> Self::Atom {
        self.value.subtendril(offset, length)
    }
}

impl<'arena> crate::attribute::Attributes<'arena> for Attributes<'arena> {
    type Attribute = Attribute;

    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn item(&self, index: usize) -> Option<cell::Ref<'arena, Self::Attribute>> {
        cell::Ref::filter_map(self.0.borrow(), |v| v.get(index)).ok()
    }

    fn item_mut(&self, index: usize) -> Option<RefMut<'arena, Self::Attribute>> {
        RefMut::filter_map(self.0.borrow_mut(), |v| v.get_mut(index)).ok()
    }

    fn get_named_item(
        &self,
        name: &<Self::Attribute as crate::attribute::Attr>::Name,
    ) -> Option<cell::Ref<'arena, Self::Attribute>> {
        cell::Ref::filter_map(self.0.borrow(), |v| {
            v.iter()
                .find(|a| a.prefix() == name.prefix() && a.local_name() == name.local_name())
        })
        .ok()
    }

    fn get_named_item_mut(
        &self,
        name: &<Self::Attribute as crate::attribute::Attr>::Name,
    ) -> Option<RefMut<'arena, Self::Attribute>> {
        RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut()
                .find(|a| a.prefix() == name.prefix() && a.local_name() == name.local_name())
        })
        .ok()
    }

    fn get_named_item_local(
        &self,
        name: &<<Self::Attribute as crate::attribute::Attr>::Name as Name>::LocalName,
    ) -> Option<cell::Ref<'arena, Self::Attribute>> {
        cell::Ref::filter_map(self.0.borrow(), |v| {
            v.iter()
                .find(|a| a.prefix().is_none() && a.local_name() == name)
        })
        .ok()
    }

    fn get_named_item_local_mut(
        &self,
        name: &<<Self::Attribute as crate::attribute::Attr>::Name as Name>::LocalName,
    ) -> Option<RefMut<'arena, Self::Attribute>> {
        RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut()
                .find(|a| a.prefix().is_none() && a.local_name() == name)
        })
        .ok()
    }

    fn get_named_item_ns(
        &self,
        namespace: &<<Self::Attribute as crate::attribute::Attr>::Name as Name>::Namespace,
        name: &<<Self::Attribute as crate::attribute::Attr>::Name as Name>::LocalName,
    ) -> Option<cell::Ref<'arena, Self::Attribute>> {
        cell::Ref::filter_map(self.0.borrow(), |v| {
            v.iter()
                .find(|a| a.local_name() == name && a.name().ns() == namespace)
        })
        .ok()
    }

    fn remove_named_item(
        &self,
        name: &<Self::Attribute as crate::attribute::Attr>::Name,
    ) -> Option<Self::Attribute> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs
            .iter()
            .position(|a| a.prefix() == name.prefix() && a.local_name() == name.local_name())?;
        Some(attrs.remove(index))
    }

    fn remove_named_item_local(
        &self,
        name: &<<Self::Attribute as crate::attribute::Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs
            .iter()
            .position(|a| a.prefix().is_none() && a.local_name() == name)?;
        Some(attrs.remove(index))
    }

    fn set_named_item(&self, attr: Self::Attribute) -> Option<Self::Attribute> {
        let attrs = &mut *self.0.borrow_mut();
        if let Some(index) = attrs
            .iter()
            .position(|a| a.prefix() == attr.prefix() && a.local_name() == attr.local_name())
        {
            Some(std::mem::replace(&mut attrs[index], attr))
        } else {
            attrs.push(attr);
            None
        }
    }

    fn sort(&self, order: &[String], xmlns_front: bool) {
        fn get_ns_priority<N: Name>(name: &N, xmlns_front: bool) -> usize {
            if xmlns_front {
                if name.prefix().is_none() && name.local_name().as_ref() == "xmlns" {
                    return 3;
                }
                if name
                    .prefix()
                    .as_ref()
                    .is_some_and(|p| p.as_ref() == "xmlns")
                {
                    return 2;
                }
            }
            if name.prefix().is_some() {
                return 1;
            }
            0
        }

        self.0.borrow_mut().sort_by(|a, b| {
            let a_priority = get_ns_priority(a.name(), xmlns_front);
            let b_priority = get_ns_priority(b.name(), xmlns_front);
            let priority_ord = b_priority.cmp(&a_priority);
            if priority_ord != std::cmp::Ordering::Equal {
                return priority_ord;
            }

            let a_part = a
                .local_name()
                .split_once('-')
                .map_or_else(|| a.local_name().as_ref(), |p| p.0);
            let b_part = b
                .local_name()
                .split_once('-')
                .map_or_else(|| b.local_name().as_ref(), |p| p.0);
            if a_part != b_part {
                let a_in_order = order.iter().position(|x| x == a_part);
                let b_in_order = order.iter().position(|x| x == b_part);
                if a_in_order.is_some() && b_in_order.is_some() {
                    return a_in_order.cmp(&b_in_order);
                }
                if a_in_order.is_some() {
                    return std::cmp::Ordering::Less;
                }
                if b_in_order.is_some() {
                    return std::cmp::Ordering::Greater;
                }
            }

            a.name().cmp(b.name())
        });
    }

    fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&Self::Attribute) -> bool,
    {
        self.0.borrow_mut().retain(|attr| f(attr));
    }
}

impl Debug for Attributes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Attributes5Ever { ")?;
        self.0.borrow().iter().try_for_each(|a| {
            f.write_fmt(format_args!(r#"{}="{}" "#, a.name().formatter(), a.value()))
        })?;
        f.write_str("} ")
    }
}

impl<'arena> ClassList<'arena> {
    fn get_token_range(
        &self,
        token: &<<Self as crate::class_list::ClassList>::Attribute as crate::attribute::Attr>::Atom,
    ) -> Option<(u32, u32)> {
        let attr = self.attr()?;

        let mut start = 0;
        let mut end = 0;
        let mut skip_to_next_word = false;
        let mut saw_whitespace = false;
        for (i, char) in attr.value().chars().enumerate() {
            if saw_whitespace && !char.is_whitespace() {
                skip_to_next_word = false;
                saw_whitespace = false;
                start = i;
                end = i;
            } else if char.is_whitespace() {
                if end - start == token.len() {
                    break;
                }
                saw_whitespace = true;
                continue;
            }
            if skip_to_next_word {
                continue;
            }
            if token.chars().nth(end - start).is_some_and(|c| c == char) {
                end = i + 1;
                continue;
            }
            skip_to_next_word = true;
        }
        if end - start < token.len() || skip_to_next_word {
            return None;
        }
        Some((start as u32, end as u32))
    }

    fn attr(
        &'arena self,
    ) -> Option<RefMut<'arena, <Self as crate::class_list::ClassList>::Attribute>> {
        self.attr_by_memo().or_else(|| self.attr_by_search())
    }

    fn attr_by_memo(
        &self,
    ) -> Option<RefMut<'arena, <Self as crate::class_list::ClassList>::Attribute>> {
        let attrs = self.attrs.0.borrow_mut();
        let index = self.class_index_memo.get();
        let option = RefMut::filter_map(attrs, |a| a.get_mut(index)).ok();
        if option
            .as_ref()
            .is_some_and(|a| a.prefix().is_none() && a.local_name().as_ref() == "class")
        {
            return option;
        }
        None
    }

    fn attr_by_search(
        &self,
    ) -> Option<RefMut<'arena, <Self as crate::class_list::ClassList>::Attribute>> {
        let attrs = self.attrs.0.borrow_mut();
        RefMut::filter_map(attrs, |a| {
            let (i, attr) = a
                .iter_mut()
                .enumerate()
                .find(|(_, a)| a.prefix().is_none() && a.local_name().as_ref() == "class")?;
            self.class_index_memo.set(i);
            Some(attr)
        })
        .ok()
    }
}

impl crate::class_list::ClassList for ClassList<'_> {
    type Attribute = Attribute;

    fn length(&self) -> usize {
        self.tokens.len()
    }

    fn value(&self) -> <Self::Attribute as crate::attribute::Attr>::Atom {
        self.attr().map(|a| a.value().clone()).unwrap_or_default()
    }

    fn add(&mut self, token: <Self::Attribute as crate::attribute::Attr>::Atom) {
        use crate::attribute::Attributes;

        if self.contains(&token) {
            return;
        };
        let Some(mut attr) = self.attr() else {
            self.attrs.set_named_item(crate::attribute::Attr::new(
                Name::new(None, "class".into()),
                token.clone(),
            ));
            self.tokens.push(token);
            return;
        };

        attr.push(&token);
    }

    fn contains(&self, token: &<Self::Attribute as crate::attribute::Attr>::Atom) -> bool {
        self.tokens.contains(token)
    }

    fn item(&self, index: usize) -> Option<&<Self::Attribute as crate::attribute::Attr>::Atom> {
        self.tokens.get(index)
    }

    fn remove(&mut self, token: &<Self::Attribute as crate::attribute::Attr>::Atom) {
        use crate::attribute::Attributes;

        let Some(index) = self.tokens.iter().position(|t| t == token) else {
            log::debug!("class not removed, not present in token memo");
            return;
        };
        self.tokens.remove(index);

        let Some((start, end)) = self.get_token_range(token) else {
            log::debug!("class not removed, not present in actual attrubute");
            return;
        };

        let mut attr = self.attr().expect("had token");
        let new_value = attr.sub_value(0, start);
        let new_end = attr.sub_value(end, attr.value().len() as u32 - end);
        if new_value.trim().is_empty() {
            drop(attr);
            self.attrs.remove_named_item_local(&"class".into());
        } else {
            attr.set_value(new_value);
            attr.push(&new_end);
        }
    }

    fn replace(
        &mut self,
        old_token: <Self::Attribute as crate::attribute::Attr>::Atom,
        new_token: <Self::Attribute as crate::attribute::Attr>::Atom,
    ) -> bool {
        let Some(index) = self.tokens.iter().position(|t| t == &old_token) else {
            return false;
        };

        let Some((start, end)) = self.get_token_range(&old_token) else {
            return false;
        };

        let mut attr = self.attr().expect("had token");
        let new_value = attr.sub_value(0, start);
        let new_end = attr.sub_value(end, attr.value().len() as u32 - end);
        attr.set_value(new_value);
        attr.push(&new_token);
        attr.push(&new_end);
        drop(attr);
        self.tokens[index] = new_token;

        true
    }

    fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = &<Self::Attribute as crate::attribute::Attr>::Atom> {
        self.tokens.iter()
    }
}

impl NodeData {
    fn node_type(&self) -> node::Type {
        match self {
            Self::Root | Self::Document => node::Type::Document,
            Self::Element { .. } => node::Type::Element,
            Self::PI { .. } => node::Type::ProcessingInstruction,
            Self::Text { .. } => node::Type::Text,
            Self::Comment(..) => node::Type::Comment,
        }
    }

    fn name(&self) -> StrTendril {
        match self {
            Self::Comment { .. } => "#comment".into(),
            Self::Document | Self::Root => "#document".into(),
            Self::Element { name, .. } => name.borrow().local.borrow().to_uppercase().into(),
            Self::PI { target, .. } => target.clone(),
            Self::Text { .. } => "#text".into(),
        }
    }

    fn value(&self) -> Option<StrTendril> {
        match &self {
            Self::Comment(value) | Self::Text(value) | Self::PI { value, .. } => {
                value.borrow().clone()
            }
            _ => None,
        }
    }

    fn processing_instruction(&self) -> Option<(StrTendril, StrTendril)> {
        match self {
            NodeData::PI { target, value } => {
                Some((target.clone(), value.borrow().as_ref().unwrap().clone()))
            }
            _ => None,
        }
    }

    fn try_set_node_value(&self, value: StrTendril) -> Option<()> {
        match self {
            Self::Text(old_value) => {
                old_value.replace(Some(value));
                Some(())
            }
            _ => None,
        }
    }
}

impl<'arena> Node<'arena> {
    fn text_content_recursive(&'arena self) -> Option<StrTendril> {
        match &self.node_data {
            NodeData::Text(value) | NodeData::Comment(value) | NodeData::PI { value, .. } => {
                value.borrow().clone()
            }
            NodeData::Document | NodeData::Root => None,
            NodeData::Element { .. } => Some(
                self.child_nodes_iter()
                    .filter_map(Self::text_content_recursive)
                    .fold(StrTendril::default(), |mut acc, item| {
                        acc.push_tendril(&item);
                        acc
                    }),
            ),
        }
    }

    /// Creates a clean node with the given node data.
    pub fn new(data: NodeData) -> Self {
        Self {
            parent: Cell::new(None),
            next_sibling: Cell::new(None),
            previous_sibling: Cell::new(None),
            first_child: Cell::new(None),
            last_child: Cell::new(None),
            node_data: data,
        }
    }
}

impl<'arena> node::Node<'arena> for Ref<'arena> {
    type Arena = Arena<'arena>;
    type Atom = StrTendril;
    type Child = Ref<'arena>;
    type ParentChild = Ref<'arena>;
    type Parent = Element<'arena>;

    fn ptr_eq(&self, other: &impl node::Node<'arena>) -> bool {
        self.as_ptr_byte() == other.as_ptr_byte()
    }

    fn as_ptr_byte(&self) -> usize {
        &raw const *self as usize
    }

    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self::Child> {
        ChildNodes {
            front: self.first_child(),
            end: self.last_child(),
        }
    }

    #[allow(refining_impl_trait)]
    fn element(&self) -> Option<Element<'arena>> {
        match self.node_type() {
            node::Type::Element | node::Type::Document => Element::new(*self),
            _ => None,
        }
    }

    fn empty(&self) {
        self.first_child.set(None);
        self.last_child.set(None);
    }

    fn find_element(&self) -> Option<impl element::Element<'arena>> {
        <Element as element::Element<'arena>>::find_element(*self)
    }

    fn retain_children<F>(&self, mut f: F)
    where
        F: FnMut(Self::Child) -> bool,
    {
        let mut current = self.first_child.get();
        let mut is_first = true;
        while let Some(child) = current {
            current = child.next_sibling.get();
            let retain = f(child);
            if retain {
                is_first = false;
                continue;
            }

            if is_first {
                self.first_child.set(child.next_sibling.get());
            } else {
                let prev_child = child
                    .previous_sibling
                    .take()
                    .expect("non-first child should have previous child");
                let next_child = child.next_sibling.take();
                prev_child.next_sibling.set(next_child);
            }
        }
    }

    fn node_type(&self) -> node::Type {
        self.node_data.node_type()
    }

    fn parent_node(&self) -> Option<Self::Parent> {
        self.parent.get().and_then(Element::new)
    }

    fn set_parent_node(&self, new_parent: &Self::Parent) -> Option<Self::Parent> {
        self.parent
            .replace(Some(new_parent.node))
            .and_then(Element::new)
    }

    fn append_child(&self, a_child: Self::Child) {
        if let Some(child) = self.last_child.get() {
            child.next_sibling.set(Some(a_child));
            self.last_child.set(Some(a_child));
        } else {
            self.first_child.set(Some(a_child));
            self.last_child.set(Some(a_child));
        }
    }

    fn item(&self, index: usize) -> Option<Self::Child> {
        self.child_nodes_iter().nth(index)
    }

    fn node_name(&self) -> Self::Atom {
        self.node_data.name()
    }

    fn node_value(&self) -> Option<Self::Atom> {
        self.node_data.value()
    }

    fn processing_instruction(&self) -> Option<(Self::Atom, Self::Atom)> {
        self.node_data.processing_instruction()
    }

    fn try_set_node_value(&self, value: Self::Atom) -> Option<()> {
        self.node_data.try_set_node_value(value)
    }

    fn text_content(&self) -> Option<Self::Atom> {
        if !self.is_empty() {
            return self.text_content_recursive();
        }
        match &self.node_data {
            NodeData::Document | NodeData::Root => None,
            NodeData::Text(value) | NodeData::Comment(value) | NodeData::PI { value, .. } => {
                value.borrow().clone()
            }
            NodeData::Element { .. } => Some(StrTendril::default()),
        }
    }

    fn set_text_content(&self, content: Self::Atom, arena: &Self::Arena) {
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

    fn text(&self, content: Self::Atom, arena: &Self::Arena) -> Self::Child {
        arena.alloc(Node::new(NodeData::Text(RefCell::new(Some(content)))))
    }

    fn remove(&self) {
        let previous_sibling = self.previous_sibling.take();
        let next_sibling = self.next_sibling.take();
        let parent = self.parent.take();
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
    }

    fn remove_child_at(&mut self, index: usize) -> Option<Self::Child> {
        let child = self.child_nodes_iter().nth(index);
        child?.remove();
        child
    }

    fn clone_node(&self) -> Self {
        todo!("needs arena")
    }

    fn replace_child(
        &mut self,
        new_child: Self::Child,
        // TODO: remove ref
        old_child: &Self::Child,
    ) -> Option<Self::Child> {
        let child = self
            .child_nodes_iter()
            .find(|child| child.ptr_eq(old_child))?;
        let previous_sibling = child.previous_sibling.take();
        let next_sibling = child.next_sibling.take();
        let parent = child.parent.take();
        if let Some(previous_sibling) = previous_sibling {
            previous_sibling.next_sibling.set(Some(new_child));
        } else if let Some(parent) = parent {
            parent.first_child.set(Some(new_child));
        }
        if let Some(next_sibling) = next_sibling {
            next_sibling.previous_sibling.set(Some(new_child));
        } else if let Some(parent) = parent {
            parent.last_child.set(Some(new_child));
        }
        Some(*old_child)
    }

    // TODO: deprecate
    fn to_owned(&self) -> Self {
        self
    }

    fn as_child(&self) -> Self::Child {
        self
    }

    fn as_impl(&self) -> impl node::Node<'arena> {
        *self
    }

    fn as_parent_child(&self) -> Self::ParentChild {
        self
    }
}

impl Debug for Ref<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = &self.node_data;
        let mut child = self.first_child.get();
        let mut child_len = 0;
        while let Some(current_child) = child {
            child_len += 1;
            child = current_child.next_sibling.get();
        }
        f.write_fmt(format_args!(
            "Node {{
    data: {data:?}
    children: {child_len}
}}"
        ))
    }
}

impl Element<'_> {
    fn data(&self) -> ElementData {
        if let NodeData::Element { name, attrs, .. } = &self.node.node_data {
            ElementData { name, attrs }
        } else {
            unreachable!("Element contains non-element data. This is a bug!")
        }
    }
}

impl<'arena> node::Node<'arena> for Element<'arena> {
    type Arena = Arena<'arena>;
    type Atom = StrTendril;
    type Child = Ref<'arena>;
    type ParentChild = Ref<'arena>;
    type Parent = Element<'arena>;

    fn ptr_eq(&self, other: &impl node::Node<'arena>) -> bool {
        self.node.ptr_eq(other)
    }

    fn as_ptr_byte(&self) -> usize {
        self.node.as_ptr_byte()
    }

    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self::Child> {
        self.node.child_nodes_iter()
    }

    fn element(&self) -> Option<impl element::Element<'arena>> {
        Some(self.clone())
    }

    fn empty(&self) {
        self.node.empty();
    }

    fn find_element(&self) -> Option<impl element::Element<'arena>> {
        Some(self.clone())
    }

    fn retain_children<F>(&self, f: F)
    where
        F: FnMut(Self::Child) -> bool,
    {
        self.node.retain_children(f);
    }

    fn node_type(&self) -> node::Type {
        self.node.node_type()
    }

    fn parent_node(&self) -> Option<Self::Parent> {
        self.node.parent_node()
    }

    fn set_parent_node(&self, new_parent: &Self::Parent) -> Option<Self::Parent> {
        self.node.set_parent_node(new_parent)
    }

    fn append_child(&self, a_child: Self::Child) {
        self.node.append_child(a_child);
    }

    fn item(&self, index: usize) -> Option<Self::Child> {
        self.node.item(index)
    }

    fn node_name(&self) -> Self::Atom {
        self.node.node_name()
    }

    fn node_value(&self) -> Option<Self::Atom> {
        None
    }

    fn processing_instruction(&self) -> Option<(Self::Atom, Self::Atom)> {
        None
    }

    fn try_set_node_value(&self, _value: Self::Atom) -> Option<()> {
        None
    }

    fn text_content(&self) -> Option<Self::Atom> {
        self.node.text_content()
    }

    fn set_text_content(&self, content: Self::Atom, arena: &Self::Arena) {
        self.node.set_text_content(content, arena);
    }

    fn text(&self, content: Self::Atom, arena: &Self::Arena) -> Self::Child {
        self.node.text(content, arena)
    }

    fn remove(&self) {
        self.node.remove();
    }

    fn remove_child_at(&mut self, index: usize) -> Option<Self::Child> {
        self.node.remove_child_at(index)
    }

    fn clone_node(&self) -> Self {
        Self::new(self.node.clone_node()).unwrap()
    }

    fn replace_child(
        &mut self,
        new_child: Self::Child,
        old_child: &Self::Child,
    ) -> Option<Self::Child> {
        self.node.replace_child(new_child, old_child)
    }

    fn to_owned(&self) -> Self {
        self.clone()
    }

    fn as_child(&self) -> Self::Child {
        self.node.as_child()
    }

    fn as_impl(&self) -> impl node::Node<'arena> {
        self.node.as_impl()
    }

    fn as_parent_child(&self) -> Self::ParentChild {
        self.node.as_parent_child()
    }
}

impl<'arena> element::Element<'arena> for Element<'arena> {
    type Name = QualName;
    type Attributes<'a> = Attributes<'a>;
    type Attr = Attribute;

    fn new(node: Ref<'arena>) -> Option<Self> {
        if !matches!(node.node_type(), node::Type::Element | node::Type::Document) {
            return None;
        }
        cfg_if! {
            if #[cfg(feature = "selectors")] {
                Some(Self {
                    node,
                })
            } else {
                Some(Self { node })
            }
        }
    }

    fn as_document(&self) -> impl crate::document::Document<'arena, Root = Self> {
        Document(self.clone())
    }

    fn from_parent(node: Self::ParentChild) -> Option<Self> {
        Self::new(node)
    }

    fn tag_name(&self) -> Self::Atom {
        self.node.node_name()
    }

    fn qual_name(&self) -> &Self::Name {
        self.data().name
    }

    fn replace_children(&self, children: Vec<Self::Child>) {
        self.node.first_child.set(children.first().copied());
        self.node.last_child.set(children.last().copied());
        for i in 0..children.len() {
            let current = children.get(i).expect("`i` should be within len");
            current.parent.set(Some(self.node));
            current.previous_sibling.set(children.get(i - 1).copied());
            current.next_sibling.set(children.get(i + 1).copied());
        }
    }

    fn set_local_name(&self, new_name: <Self::Name as Name>::LocalName, arena: &Self::Arena) {
        let NodeData::Element { attrs, .. } = &self.node.node_data else {
            panic!("expected an element!");
        };
        let replacement = arena.alloc(Node::new(NodeData::Element {
            name: QualName::new(None, new_name),
            attrs: attrs.clone(),
            #[cfg(feature = "selectors")]
            selector_flags: Cell::new(None),
        }));
        self.replace_with(replacement);
    }

    fn append(&self, node: Self::Child) {
        if let Some(last_node) = self.node.last_child.get() {
            last_node.next_sibling.set(Some(node));
            self.node.last_child.set(Some(node));
        } else {
            debug_assert!(self.node.first_child.get().is_none());
            self.node.first_child.set(Some(node));
            self.node.last_child.set(Some(node));
        }
    }

    fn attributes(&self) -> Self::Attributes<'_> {
        Attributes(self.data().attrs)
    }

    fn set_attributes(&self, new_attrs: Self::Attributes<'_>) {
        let attrs = self.data().attrs;
        attrs.replace(new_attrs.0.take());
    }

    fn parent_element(&self) -> Option<Self> {
        self.node.parent_node()
    }

    fn class_list(
        &self,
    ) -> impl crate::class_list::ClassList<
        Attribute = <Self::Attributes<'_> as crate::attribute::Attributes>::Attribute,
    > {
        ClassList {
            attrs: self.attributes(),
            class_index_memo: Cell::new(0),
            tokens: self
                .attributes()
                .get_named_item_local(&local_name!("class"))
                .map(|a| a.value().split_whitespace().map(Into::into).collect())
                .unwrap_or_default(),
        }
    }

    fn document(&self) -> Option<Self> {
        let parent = self.parent_node()?;
        match self.node.node_data {
            NodeData::Element { .. } => parent.document(),
            NodeData::Document | NodeData::Root => Some(parent),
            _ => None,
        }
    }

    fn flatten(&self) {
        let parent = self.node.parent.take();
        let current = self.node.first_child.get();
        while let Some(current) = current {
            current.parent.set(parent);
        }

        let previous_sibling = self.node.previous_sibling.take();
        let next_sibling = self.node.next_sibling.take();
        let first_child = self.node.first_child.take();
        let last_child = self.node.last_child.take();

        if let Some(first_child) = first_child {
            if let Some(previous_sibling) = previous_sibling {
                previous_sibling.next_sibling.set(Some(first_child));
            } else if let Some(parent) = parent {
                parent.first_child.set(Some(first_child));
            }
        } else if let Some(previous_sibling) = previous_sibling {
            previous_sibling.next_sibling.set(next_sibling);
        } else if let Some(parent) = parent {
            parent.first_child.set(next_sibling);
        }
        if let Some(last_child) = last_child {
            if let Some(next_sibling) = next_sibling {
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

    fn find_element(node: <Self as node::Node<'arena>>::ParentChild) -> Option<Self> {
        let mut queue = VecDeque::new();
        queue.push_back(node);

        while let Some(current) = queue.pop_front() {
            let maybe_element = current.element();
            if maybe_element.is_some() {
                return maybe_element;
            }

            for child in current.child_nodes_iter() {
                queue.push_back(child);
            }
        }
        None
    }

    fn sort_child_elements<F>(&self, mut f: F)
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

        self.node.first_child.set(children.first().copied());
        self.node.last_child.set(children.last().copied());
        for i in 0..children.len() {
            let child = children[i];
            child.previous_sibling.set(children.get(i - 1).copied());
            child.next_sibling.set(children.get(i + 1).copied());
        }
    }

    #[cfg(feature = "selectors")]
    fn set_selector_flags(&self, flags: selectors::matching::ElementSelectorFlags) {
        let NodeData::Element {
            ref selector_flags, ..
        } = self.node.node_data
        else {
            return;
        };
        selector_flags.set(Some(flags));
    }
}

impl Debug for Element<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.node_type() != node::Type::Element {
            return (&self.node).fmt(f);
        }
        let name = self.qual_name().formatter();
        let attributes = self.attributes();
        let text = self.text_content().map(|s| s.trim().to_string());
        let child_count = match (&self.node).child_node_count() {
            0 => String::from("/>"),
            len => format!(">{len} child nodes</{name}>"),
        };
        f.write_fmt(format_args!(
            r"Element {{ <{name} {attributes:?}{child_count} {text:?} }}"
        ))
    }
}

impl std::hash::Hash for Element<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr_byte().hash(state);
    }
}

impl Eq for Element<'_> {}

impl PartialEq for Element<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<'arena> document::Document<'arena> for Document<'arena> {
    type Root = Element<'arena>;

    fn document_element(&self) -> &Self::Root {
        &self.0
    }

    fn create_c_data_section(
        &self,
        data: <Self::Root as node::Node<'arena>>::Atom,
        arena: &<Self::Root as node::Node<'arena>>::Arena,
    ) -> <Self::Root as node::Node<'arena>>::Child {
        self.create_text_node(data, arena)
    }

    fn create_element(
        &self,
        tag_name: <Self::Root as element::Element<'arena>>::Name,
        arena: &<Self::Root as node::Node<'arena>>::Arena,
    ) -> Self::Root {
        Element::new(arena.alloc(Node::new(NodeData::Element {
            name: tag_name,
            attrs: RefCell::new(vec![]),
            #[cfg(feature = "selectors")]
            selector_flags: Cell::new(None),
        })))
        .expect("created element should be an element")
    }

    fn create_processing_instruction(
        &self,
        target: <Self::Root as node::Node<'arena>>::Atom,
        data: <Self::Root as node::Node<'arena>>::Atom,
        arena: &<Self::Root as node::Node<'arena>>::Arena,
    ) -> <<Self::Root as node::Node<'arena>>::Child as node::Node<'arena>>::ParentChild {
        arena.alloc(Node::new(NodeData::PI {
            target,
            value: RefCell::new(Some(data)),
        }))
    }

    fn create_text_node(
        &self,
        data: <Self::Root as node::Node<'arena>>::Atom,
        arena: <Self::Root as node::Node<'arena>>::Arena,
    ) -> <Self::Root as node::Node<'arena>>::Child {
        arena.alloc(Node::new(NodeData::Text(RefCell::new(Some(data)))))
    }
}

struct ChildNodes<'arena> {
    front: Option<Ref<'arena>>,
    end: Option<Ref<'arena>>,
}

impl<'arena> Iterator for ChildNodes<'arena> {
    type Item = Ref<'arena>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.front?;
        let next = current.next_sibling.get()?;
        self.front = Some(next);
        Some(next)
    }
}

impl DoubleEndedIterator for ChildNodes<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let current = self.end?;
        let prev = current.previous_sibling.get()?;
        self.end = Some(prev);
        Some(prev)
    }
}
