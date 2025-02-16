use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    collections::VecDeque,
    fmt::Debug,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
};

use markup5ever::{
    local_name, tendril::StrTendril, Attribute, LocalName, Namespace, Prefix, QualName,
};
use rcdom::NodeData;
use xml5ever::{namespace_url, ns};

use crate::{
    attribute::{Attr, Attributes, AttributesIter, AttributesIterMut},
    class_list::ClassList,
    document::Document,
    element::{self, Element},
    name::Name,
    node::{self, Node},
};

#[cfg(feature = "parse")]
use crate::parse;

#[cfg(feature = "serialize")]
use crate::serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualName5Ever(QualName);

#[derive(Debug)]
pub enum Attribute5Ever<'a> {
    Borrowed(RefMut<'a, Attribute>),
    Owned(Attribute),
}

#[derive(Clone)]
pub struct Attributes5Ever<'a>(pub &'a RefCell<Vec<Attribute>>);

pub struct ClassList5Ever<'a> {
    pub attrs: Attributes5Ever<'a>,
    pub class_index_memo: Cell<usize>,
    pub tokens: Vec<StrTendril>,
}

#[derive(Clone)]
pub struct Node5Ever(Rc<rcdom::Node>);

#[derive(Debug)]
pub struct Node5EverRef(Rc<Node5Ever>);

#[derive(Clone)]
pub struct Element5Ever {
    node: Node5Ever,
    #[cfg(feature = "selectors")]
    selector_flags: Cell<Option<selectors::matching::ElementSelectorFlags>>,
}

pub struct Document5Ever(Element5Ever);

impl Name for QualName {
    type LocalName = LocalName;
    type Prefix = Prefix;
    type Namespace = Namespace;

    fn new(prefix: Option<Self::Prefix>, local: Self::LocalName) -> Self {
        QualName {
            prefix,
            local,
            ns: Namespace::default(),
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

    fn is_empty(&self) -> bool {
        match self.prefix {
            Some(_) => false,
            None => self.local.is_empty(),
        }
    }

    fn len(&self) -> usize {
        match &self.prefix {
            Some(p) => p.len() + 1 + self.local.len(),
            None => self.local.len(),
        }
    }

    fn parse(value: &str) -> Self {
        let mut parts = value.split(':');
        let prefix_or_local = parts
            .next()
            .expect("Attempted to make qual-name from empty string");
        let maybe_local = parts.next();
        assert_eq!(parts.next(), None);

        match maybe_local {
            Some(local) => QualName {
                prefix: Some(prefix_or_local.into()),
                local: local.into(),
                ns: string_cache::Atom::default(),
            },
            None => QualName {
                prefix: None,
                local: prefix_or_local.into(),
                ns: string_cache::Atom::default(),
            },
        }
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

    fn presentation(&self) -> Option<crate::style::PresentationAttr> {
        self.name.prefix.as_ref()?;
        let id = crate::style::PresentationAttrId::from(self.name.local.as_ref());
        crate::style::PresentationAttr::parse_string(
            id,
            self.name.local.as_ref(),
            lightningcss::stylesheet::ParserOptions::default(),
        )
        .ok()
    }
}

impl<'a> Attributes<'a> for Attributes5Ever<'a> {
    type Attribute = Attribute;
    type Deref = Ref<'a, Attribute>;
    type DerefMut = RefMut<'a, Attribute>;

    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn item(&self, index: usize) -> Option<Self::Deref> {
        Ref::filter_map(self.0.borrow(), |v| v.get(index)).ok()
    }

    fn item_mut(&self, index: usize) -> Option<Self::DerefMut> {
        RefMut::filter_map(self.0.borrow_mut(), |v| v.get_mut(index)).ok()
    }

    fn get_named_item(&self, name: &QualName) -> Option<Self::Deref> {
        Ref::filter_map(self.0.borrow(), |v| {
            v.iter()
                .find(|a| a.name.prefix == name.prefix && a.name.local == name.local)
        })
        .ok()
    }

    fn get_named_item_mut(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::DerefMut> {
        RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut()
                .find(|a| a.name.prefix == name.prefix && a.name.local == name.local)
        })
        .ok()
    }

    fn get_named_item_local(&self, name: &LocalName) -> Option<Self::Deref> {
        Ref::filter_map(self.0.borrow(), |v| {
            v.iter()
                .find(|a| a.prefix().is_none() && &a.name.local == name)
        })
        .ok()
    }

    fn get_named_item_local_mut(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::DerefMut> {
        RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut()
                .find(|a| a.prefix().is_none() && &a.name.local == name)
        })
        .ok()
    }

    fn get_named_item_ns(&self, namespace: &Namespace, name: &LocalName) -> Option<Self::Deref> {
        Ref::filter_map(self.0.borrow(), |v| {
            v.iter()
                .find(|a| &a.name.local == name && &a.name.ns == namespace)
        })
        .ok()
    }

    fn remove_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Attribute> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs
            .iter()
            .position(|a| a.name.prefix == name.prefix && a.name.local == name.local)?;
        Some(attrs.remove(index))
    }

    fn remove_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs
            .iter()
            .position(|a| a.name.prefix.is_none() && &a.name.local == name)?;
        Some(attrs.remove(index))
    }

    fn set_named_item(&self, attr: Self::Attribute) -> Option<Self::Attribute> {
        let attrs = &mut *self.0.borrow_mut();
        if let Some(index) = attrs
            .iter()
            .position(|a| a.name.prefix == attr.name.prefix && a.name.local == attr.name.local)
        {
            Some(std::mem::replace(&mut attrs[index], attr))
        } else {
            attrs.push(attr);
            None
        }
    }

    fn set_named_item_qual(
        &self,
        name: <Self::Attribute as Attr>::Name,
        value: <Self::Attribute as Attr>::Atom,
    ) -> Option<Self::Attribute> {
        let attr = Attribute { name, value };
        self.set_named_item(attr)
    }

    fn into_iter(self) -> AttributesIter<'a, Self> {
        AttributesIter::new(self)
    }

    fn into_iter_mut(self) -> crate::attribute::AttributesIterMut<'a, Self> {
        AttributesIterMut::new(self)
    }

    fn sort(&self, order: &[String], xmlns_front: bool) {
        fn get_ns_priority(name: &QualName, xmlns_front: bool) -> usize {
            if xmlns_front {
                if name.prefix.is_none() && name.local == local_name!("xmlns") {
                    return 3;
                }
                if name.prefix.as_ref().is_some_and(|p| p == "xmlns") {
                    return 2;
                }
            }
            if name.prefix.is_some() {
                return 1;
            }
            0
        }

        self.0.borrow_mut().sort_by(|a, b| {
            let a_priority = get_ns_priority(&a.name, xmlns_front);
            let b_priority = get_ns_priority(&b.name, xmlns_front);
            let priority_ord = b_priority.cmp(&a_priority);
            if priority_ord != std::cmp::Ordering::Equal {
                return priority_ord;
            }

            let a_part = a
                .name
                .local
                .split_once('-')
                .map_or_else(|| a.name.local.as_ref(), |p| p.0);
            let b_part = b
                .name
                .local
                .split_once('-')
                .map_or_else(|| b.name.local.as_ref(), |p| p.0);
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

            a.name.cmp(&b.name)
        });
    }

    fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&Self::Attribute) -> bool,
    {
        self.0.borrow_mut().retain(|attr| f(attr));
    }
}

impl Debug for Attributes5Ever<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Attributes5Ever { ")?;
        self.0.borrow().iter().try_for_each(|a| {
            f.write_fmt(format_args!(r#"{}="{}" "#, a.name.formatter(), a.value))
        })?;
        f.write_str("} ")
    }
}

impl ClassList for ClassList5Ever<'_> {
    type Attribute = Attribute;

    fn length(&self) -> usize {
        self.tokens.len()
    }

    fn value(&self) -> <Self::Attribute as Attr>::Atom {
        self.attr().map(|a| a.value.clone()).unwrap_or_default()
    }

    fn add(&mut self, token: <Self::Attribute as Attr>::Atom) {
        if self.contains(&token) {
            return;
        };
        let Some(mut attr) = self.attr() else {
            self.attrs.set_named_item(Attribute {
                name: QualName {
                    prefix: None,
                    local: local_name!("class"),
                    ns: ns!(html),
                },
                value: token.clone(),
            });
            self.tokens.push(token);
            return;
        };

        attr.value.push_tendril(&token);
    }

    fn contains(&self, token: &<Self::Attribute as Attr>::Atom) -> bool {
        self.tokens.contains(token)
    }

    fn item(&self, index: usize) -> Option<&<Self::Attribute as Attr>::Atom> {
        self.tokens.get(index)
    }

    fn remove(&mut self, token: &<Self::Attribute as Attr>::Atom) {
        let Some(index) = self.tokens.iter().position(|t| t == token) else {
            log::debug!("class not removed, not present in token memo");
            return;
        };
        self.tokens.remove(index);

        let Some((start, end)) = self.get_token_range(token) else {
            log::debug!("class not removed, not present in actual attrubute");
            return;
        };

        let attr = self.attr().expect("had token");
        let mut new_value = attr.value.subtendril(0, start);
        new_value.push_tendril(&attr.value.subtendril(end, attr.value.len() as u32 - end));
        drop(attr);
        if new_value.trim().is_empty() {
            self.attrs.remove_named_item_local(&local_name!("class"));
        } else {
            self.attrs.set_named_item_qual(
                QualName {
                    prefix: None,
                    local: local_name!("class"),
                    ns: Namespace::default(),
                },
                new_value,
            );
        }
    }

    fn replace(
        &mut self,
        old_token: <Self::Attribute as Attr>::Atom,
        new_token: <Self::Attribute as Attr>::Atom,
    ) -> bool {
        let Some(index) = self.tokens.iter().position(|t| t == &old_token) else {
            return false;
        };

        let Some((start, end)) = self.get_token_range(&old_token) else {
            return false;
        };

        let attr = self.attr().expect("had token");
        let mut new_value = attr.value.subtendril(0, start);
        new_value.push_tendril(&new_token);
        new_value.push_tendril(&attr.value.subtendril(end, attr.value.len() as u32 - end));
        drop(attr);
        self.tokens[index] = new_token;

        self.attrs.set_named_item_qual(
            QualName {
                prefix: None,
                local: local_name!("class"),
                ns: Namespace::default(),
            },
            new_value,
        );
        true
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &<Self::Attribute as Attr>::Atom> {
        self.tokens.iter()
    }
}

impl<'a> ClassList5Ever<'a> {
    fn get_token_range(
        &self,
        token: &<<Self as ClassList>::Attribute as Attr>::Atom,
    ) -> Option<(u32, u32)> {
        let attr = self.attr()?;

        let mut start = 0;
        let mut end = 0;
        let mut skip_to_next_word = false;
        let mut saw_whitespace = false;
        for (i, char) in attr.value.chars().enumerate() {
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

    fn attr(&'a self) -> Option<RefMut<'a, Attribute>> {
        self.attr_by_memo().or_else(|| self.attr_by_search())
    }

    fn attr_by_memo(&self) -> Option<RefMut<'a, Attribute>> {
        let attrs = self.attrs.0.borrow_mut();
        let index = self.class_index_memo.get();
        let option = RefMut::filter_map(attrs, |a| a.get_mut(index)).ok();
        if option
            .as_ref()
            .is_some_and(|a| a.name.prefix.is_none() && a.name.local == local_name!("class"))
        {
            return option;
        }
        None
    }

    fn attr_by_search(&self) -> Option<RefMut<'a, Attribute>> {
        let attrs = self.attrs.0.borrow_mut();
        RefMut::filter_map(attrs, |a| {
            let (i, attr) = a
                .iter_mut()
                .enumerate()
                .find(|(_, a)| a.name.prefix.is_none() && a.name.local == local_name!("class"))?;
            self.class_index_memo.set(i);
            Some(attr)
        })
        .ok()
    }
}

impl Node5Ever {
    /// Collects the text content of the node, with the behaviour of
    /// [textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)'s
    /// recursive calls.
    ///
    /// > returns the concatenation of the textContent of every child node, excluding comments and processing instructions. (This is an empty string if the node has no children.)
    fn node_data_text_content(node: &Rc<rcdom::Node>) -> Option<String> {
        match &node.data {
            NodeData::Text { contents } => Some(contents.borrow().to_string()),
            NodeData::Doctype { .. } | NodeData::Document => None,
            NodeData::Comment { .. } | NodeData::ProcessingInstruction { .. } => {
                Some(String::new())
            }
            NodeData::Element { .. } => Some(
                node.children
                    .borrow()
                    .iter()
                    .filter_map(Node5Ever::node_data_text_content)
                    .fold(String::new(), |acc, item| acc + &item),
            ),
        }
    }

    /// Creates a deep clone of the node's data
    fn clone_node_data(&self) -> NodeData {
        match &self.0.data {
            NodeData::Comment { contents } => NodeData::Comment {
                contents: contents.clone(),
            },
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            } => NodeData::Doctype {
                name: name.clone(),
                public_id: public_id.clone(),
                system_id: system_id.clone(),
            },
            NodeData::Document => NodeData::Document,
            NodeData::ProcessingInstruction { target, contents } => {
                NodeData::ProcessingInstruction {
                    target: target.clone(),
                    contents: contents.clone(),
                }
            }
            NodeData::Text { contents } => NodeData::Text {
                contents: contents.clone(),
            },
            NodeData::Element {
                name,
                attrs,
                template_contents,
                mathml_annotation_xml_integration_point,
            } => NodeData::Element {
                name: name.clone(),
                attrs: attrs.clone(),
                template_contents: template_contents.clone(),
                mathml_annotation_xml_integration_point: *mathml_annotation_xml_integration_point,
            },
        }
    }
}

impl Node for Node5Ever {
    type Atom = StrTendril;
    type Child = Node5Ever;
    type ParentChild = Node5Ever;
    type Parent = Node5Ever;

    fn ptr_eq(&self, other: &impl Node) -> bool {
        self.as_ptr_byte() == other.as_ptr_byte()
    }

    fn as_ptr_byte(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    // fn as_ref(&self) -> Box<dyn node::Ref> {
    //     Box::new(Node5EverRef(Rc::new(self.clone())))
    // }

    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        let children = self.0.children.borrow().clone();
        children.into_iter().map(Self)
    }

    fn child_nodes(&self) -> Vec<Self::Child> {
        self.0
            .children
            .borrow()
            .iter()
            .map(|node| Self(node.clone()))
            .collect()
    }

    fn child_node_count(&self) -> usize {
        self.0.children.borrow().len()
    }

    #[allow(refining_impl_trait)]
    fn element(&self) -> Option<Element5Ever> {
        match self.node_type() {
            node::Type::Element => Element5Ever::new(Node::to_owned(self)),
            _ => None,
        }
    }

    fn empty(&self) {
        self.0.children.take();
    }

    #[allow(refining_impl_trait)]
    fn find_element(&self) -> Option<Element5Ever> {
        <Element5Ever as Element>::find_element(Node::to_owned(self))
    }

    fn for_each_child<F>(&self, mut f: F)
    where
        F: FnMut(Self),
    {
        self.0
            .children
            .borrow()
            .iter()
            .for_each(|node| f(Self(node.clone())));
    }

    fn try_for_each_child<F, E>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(Self) -> Result<(), E>,
    {
        self.0
            .children
            .borrow()
            .iter()
            .try_for_each(|node| f(Self(node.clone())))
    }

    fn any_child<F>(&self, mut f: F) -> bool
    where
        F: FnMut(Self) -> bool,
    {
        self.0
            .children
            .borrow()
            .iter()
            .any(|node| f(Self(node.clone())))
    }

    fn all_children<F>(&self, mut f: F) -> bool
    where
        F: FnMut(Self) -> bool,
    {
        self.0
            .children
            .borrow()
            .iter()
            .all(|node| f(Self(node.clone())))
    }

    fn retain_children<F>(&self, mut f: F)
    where
        F: FnMut(Self::Child) -> bool,
    {
        self.0
            .children
            .borrow_mut()
            .retain(|node| f(Self(node.clone())));
    }

    fn node_type(&self) -> node::Type {
        match self.0.data {
            NodeData::Comment { .. } => node::Type::Comment,
            NodeData::Doctype { .. } => node::Type::DocumentType,
            NodeData::Document => node::Type::Document,
            NodeData::Element { .. } => node::Type::Element,
            NodeData::ProcessingInstruction { .. } => node::Type::ProcessingInstruction,
            NodeData::Text { .. } => node::Type::Text,
        }
    }

    #[allow(refining_impl_trait)]
    fn parent_node(&self) -> Option<Node5Ever> {
        let cell = &self.0.parent;
        let parent = cell.take()?;
        let node = parent.upgrade().map(Self);
        cell.set(Some(parent));
        node
    }

    #[allow(refining_impl_trait)]
    fn set_parent_node(&self, new_parent: &Self::Parent) -> Option<Node5Ever> {
        let parent = Rc::downgrade(&new_parent.0);
        let old_parent = self.0.parent.replace(Some(parent))?;
        Some(Node5Ever(old_parent.upgrade()?))
    }

    fn append_child(&mut self, a_child: Self::Child) {
        a_child.set_parent_node(self);
        self.0.children.borrow_mut().push(a_child.0);
    }

    fn insert(&mut self, index: usize, new_node: Self::Child) {
        new_node.set_parent_node(self);
        self.0.children.borrow_mut().insert(index, new_node.0);
    }

    fn node_name(&self) -> Self::Atom {
        match &self.0.data {
            NodeData::Comment { .. } => "#comment".into(),
            NodeData::Doctype { name, .. } => name.clone(),
            NodeData::Document => "#document".into(),
            NodeData::Element { name, .. } => name.local.to_uppercase().into(),
            NodeData::ProcessingInstruction { target, .. } => target.clone(),
            NodeData::Text { .. } => "#text".into(),
        }
    }

    fn node_value(&self) -> Option<Self::Atom> {
        Some(match &self.0.data {
            NodeData::Comment { contents } | NodeData::ProcessingInstruction { contents, .. } => {
                contents.clone()
            }
            NodeData::Text { contents } => contents.borrow().clone(),
            _ => return None,
        })
    }

    fn processing_instruction(&self) -> Option<(Self::Atom, Self::Atom)> {
        match &self.0.data {
            NodeData::ProcessingInstruction { target, contents } => {
                Some((target.clone(), contents.clone()))
            }
            _ => None,
        }
    }

    fn try_set_node_value(&self, value: Self::Atom) -> Option<()> {
        match &self.0.data {
            NodeData::Text { contents } => {
                contents.replace(value);
                Some(())
            }
            _ => None,
        }
    }

    fn text_content(&self) -> Option<String> {
        if self.0.children.borrow().len() > 0 {
            return Node5Ever::node_data_text_content(&self.0);
        }
        match &self.0.data {
            NodeData::Doctype { .. } | NodeData::Document => None,
            // FIXME: Empty string should only be returned on recursive calls
            NodeData::Comment { contents } | NodeData::ProcessingInstruction { contents, .. } => {
                Some(contents.to_string())
            }
            NodeData::Text { contents } => Some(contents.borrow().to_string()),
            NodeData::Element { .. } => Some(String::new()),
        }
    }

    fn set_text_content(&mut self, content: Self::Atom) {
        match &self.0.data {
            NodeData::Text { contents } => *contents.borrow_mut() = content,
            NodeData::Element { .. } => {
                let text = self.text(content);
                self.empty();
                self.append_child(text);
            }
            _ => {}
        }
    }

    fn text(&self, content: Self::Atom) -> Self {
        Node5Ever(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            children: RefCell::new(vec![]),
            data: NodeData::Text {
                contents: RefCell::new(content),
            },
        }))
    }

    fn remove(&self) {
        let Some(mut parent) = self.parent_node() else {
            // Element already removed
            return;
        };

        parent.remove_child(self.clone());
        self.0.parent.set(None);
    }

    fn remove_child_at(&mut self, index: usize) -> Option<Self::Child> {
        let mut children = self.0.children.borrow_mut();
        if children.len() <= index {
            None
        } else {
            Some(Node5Ever(children.remove(index)))
        }
    }

    fn clone_node(&self) -> Self {
        let children = self.0.children.borrow().iter().cloned().collect();
        Self(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            data: self.clone_node_data(),
            children: RefCell::new(children),
        }))
    }

    fn replace_child(
        &mut self,
        new_child: Self::Child,
        old_child: &Self::Child,
    ) -> Option<Self::Child> {
        let index = self.child_index(old_child)?;
        Some(Node5Ever(std::mem::replace(
            &mut self.0.children.borrow_mut()[index],
            new_child.0,
        )))
    }

    fn to_owned(&self) -> Self {
        self.clone()
    }

    fn as_child(&self) -> Self::Child {
        self.clone()
    }

    fn as_impl(&self) -> impl Node {
        self.clone()
    }

    fn as_parent_child(&self) -> Self::ParentChild {
        Node::to_owned(self)
    }
}

impl Debug for Node5Ever {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = &self.0.data;
        let child_len = self.0.children.borrow().len();
        f.write_fmt(format_args!(
            "Node5Ever {{
    data: {data:?}
    children: {child_len}
}}"
        ))
    }
}

impl node::Features for Node5Ever {}

#[cfg(feature = "parse")]
impl parse::Node for Node5Ever {
    fn parse(source: &str) -> anyhow::Result<Self> {
        use xml5ever::{
            driver::{parse_document, XmlParseOpts},
            tendril::TendrilSink,
        };
        let dom: rcdom::RcDom =
            parse_document(rcdom::RcDom::default(), XmlParseOpts::default()).one(source);

        Ok(Node5Ever(dom.document))
    }

    fn parse_file(mut file: &std::fs::File) -> anyhow::Result<Self> {
        use xml5ever::{
            driver::{parse_document, XmlParseOpts},
            tendril::{ReadExt, Tendril, TendrilSink},
        };

        let mut tendril = Tendril::new();
        file.read_to_tendril(&mut tendril)?;

        let mut parser =
            parse_document(rcdom::RcDom::default(), XmlParseOpts::default()).from_utf8();
        parser.process(tendril);
        let dom: rcdom::RcDom = parser.finish();

        Ok(Node5Ever(dom.document))
    }

    fn parse_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        Self::parse_file(&file)
    }
}

#[cfg(feature = "serialize")]
struct SerializableHandle {
    handle: Node5Ever,
}

#[cfg(feature = "serialize")]
enum SerializeOp {
    Open(Node5Ever),
    Close(QualName),
}

#[cfg(feature = "serialize")]
impl crate::serialize::Serialize for SerializableHandle {
    fn serialize<Wr: std::io::Write>(
        &self,
        serializer: &mut crate::serialize::Serializer<Wr>,
        traversal_scope: xml5ever::serialize::TraversalScope,
    ) -> std::io::Result<()> {
        use xml5ever::serialize::TraversalScope;
        let mut ops = VecDeque::new();
        match traversal_scope {
            TraversalScope::IncludeNode => ops.push_back(SerializeOp::Open(self.handle.clone())),
            TraversalScope::ChildrenOnly(_) => ops.extend(
                self.handle
                    .child_nodes()
                    .iter()
                    .map(|h| SerializeOp::Open(h.clone())),
            ),
        };

        while let Some(op) = ops.pop_front() {
            match op {
                SerializeOp::Open(handle) => {
                    match handle.0.data {
                        NodeData::Element {
                            ref name,
                            ref attrs,
                            ..
                        } => {
                            let children = handle.0.children.borrow();
                            let is_empty = children.is_empty()
                                || children.iter().all(|e| match e.data {
                                    NodeData::Text { ref contents } => {
                                        contents.borrow().trim().is_empty()
                                    }
                                    _ => false,
                                });
                            serializer.start_elem(
                                name,
                                attrs.borrow().iter().map(|a| (&a.name, &a.value[..])),
                                is_empty,
                            )?;
                            if is_empty {
                                continue;
                            }

                            let child_len = handle.0.children.borrow().len();
                            ops.reserve(1 + child_len);
                            if child_len > 0 {
                                ops.push_front(SerializeOp::Close(name.clone()));
                            }

                            for child in handle.0.children.borrow().iter().rev() {
                                ops.push_front(SerializeOp::Open(Node5Ever(child.clone())));
                            }
                        }
                        NodeData::Doctype { ref name, .. } => serializer.write_doctype(name)?,
                        NodeData::Text { ref contents, .. } => {
                            serializer.write_text(&contents.borrow())?;
                        }
                        NodeData::Comment { ref contents } => serializer.write_comment(contents)?,
                        NodeData::ProcessingInstruction {
                            ref target,
                            ref contents,
                        } => serializer.write_processing_instruction(target, contents)?,
                        NodeData::Document => panic!("Can't serialize Document node itself"),
                    };
                }
                SerializeOp::Close(name) => {
                    serializer.end_elem(&name)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "serialize")]
impl serialize::Node for Node5Ever {
    fn serialize(&self) -> anyhow::Result<String> {
        self.serialize_with_options(crate::serialize::Options::new())
    }

    fn serialize_with_options(&self, options: serialize::Options) -> anyhow::Result<String> {
        let mut sink: std::io::BufWriter<_> = std::io::BufWriter::new(Vec::new());
        crate::serialize::with_options(
            &mut sink,
            &SerializableHandle {
                handle: self.clone(),
            },
            xml5ever::serialize::SerializeOpts::default(),
            options,
        )?;

        let sink: Vec<_> = sink.into_inner()?;
        Ok(String::from_utf8_lossy(&sink).to_string())
    }

    fn serialize_into<Wr: std::io::Write>(&self, sink: Wr) -> anyhow::Result<()> {
        use xml5ever::serialize::SerializeOpts;

        Ok(crate::serialize::serialize(
            sink,
            &SerializableHandle {
                handle: self.clone(),
            },
            SerializeOpts::default(),
        )?)
    }
}

impl Element for Element5Ever {
    type Name = QualName;
    type Attributes<'a> = Attributes5Ever<'a>;
    type Attr = Attribute;

    fn new(node: Node5Ever) -> Option<Self> {
        if !matches!(node.node_type(), node::Type::Element | node::Type::Document) {
            return None;
        }
        Some(Self {
            node,
            #[cfg(feature = "selectors")]
            selector_flags: Cell::new(None),
        })
    }

    fn as_document(&self) -> impl crate::document::Document<Root = Self> {
        Document5Ever(self.clone())
    }

    fn from_parent(node: Node5Ever) -> Option<Self> {
        Self::new(node)
    }

    fn tag_name(&self) -> Self::Atom {
        self.node.node_name()
    }

    fn qual_name(&self) -> &Self::Name {
        self.data().name
    }

    fn replace_children(&self, children: Vec<Self::Child>) {
        let mut old_children = self.node.0.children.borrow_mut();
        for child in old_children.iter() {
            child.parent.replace(None);
        }
        old_children.iter().for_each(|c| {
            c.parent.replace(None);
        });
        for child in &children {
            child.set_parent_node(&self.node);
        }
        *old_children = children.into_iter().map(|c| c.0).collect();
    }

    fn set_local_name(&mut self, new_name: <Self::Name as Name>::LocalName) {
        let mut data = self.node.clone_node_data();
        if let rcdom::NodeData::Element { name, .. } = &mut data {
            name.local = new_name;
        };
        let clone = Node5Ever(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            children: self.node.0.children.clone(),
            data,
        }));
        self.replace_with(clone);
    }

    fn append(&self, node: Self::Child) {
        self.node.0.children.borrow_mut().push(node.0);
    }

    fn attributes(&self) -> Self::Attributes<'_> {
        Attributes5Ever(self.data().attrs)
    }

    fn set_attributes(&self, new_attrs: Self::Attributes<'_>) {
        let rcdom::NodeData::Element { attrs, .. } = &self.node.0.data else {
            unreachable!()
        };
        attrs.replace(new_attrs.0.take());
    }

    fn parent_element(&self) -> Option<Self> {
        let parent_node: Node5Ever = self.parent_node()?;
        Self::new(parent_node)
    }

    #[allow(refining_impl_trait)]
    fn class_list(&self) -> ClassList5Ever {
        ClassList5Ever {
            attrs: self.attributes(),
            class_index_memo: Cell::new(0),
            tokens: self
                .attributes()
                .get_named_item_local(&local_name!("class"))
                .as_ref()
                .map(|a| a.value().split_whitespace().map(Into::into).collect())
                .unwrap_or_default(),
        }
    }

    fn has_class(&self, token: &Self::Atom) -> bool {
        let token = token.trim_start_matches('.');
        self.class_list().contains(&token.into())
    }

    fn document(&self) -> Option<Self> {
        let parent: Node5Ever = self.parent_node()?;
        match parent.0.data {
            NodeData::Element { .. } => parent.element()?.document(),
            NodeData::Document => Some(Element5Ever {
                node: parent,
                selector_flags: Cell::new(None),
            }),
            _ => None,
        }
    }

    fn flatten(&self) {
        let children = self.node.0.children.take();

        let parent = self.parent_element();
        let Some(parent) = parent else {
            return;
        };
        self.node.0.parent.replace(None);

        let index = parent.child_index(&self.node);
        let Some(index) = index else {
            return;
        };

        for child in &children {
            child.parent.replace(Some(Rc::downgrade(&parent.node.0)));
        }

        let mut siblings = parent.node.0.children.borrow_mut();
        siblings.splice(index..=index, children);
    }

    /// Runs a breadth-first search to get the first element of a node.
    fn find_element(node: <Self as Node>::ParentChild) -> Option<Self> {
        let mut queue = VecDeque::new();
        queue.push_back(node);

        while let Some(current) = queue.pop_front() {
            let maybe_element = current.element();
            if maybe_element.is_some() {
                return maybe_element;
            }

            for child in current.child_nodes() {
                queue.push_back(child);
            }
        }
        None
    }

    #[allow(refining_impl_trait)]
    fn get_attribute_node<'a>(
        &'a self,
        attr_name: &<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> Option<Ref<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item(attr_name)
    }

    fn get_attribute_node_mut<'a>(
        &'a self,
        attr_name: &<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> Option<impl std::ops::DerefMut<Target = <Self::Attributes<'a> as Attributes<'a>>::Attribute>>
    {
        self.attributes().get_named_item_mut(attr_name)
    }

    #[allow(refining_impl_trait)]
    fn get_attribute_node_local<'a>(
        &'a self,
        attr_name: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item_local(attr_name)
    }

    #[allow(refining_impl_trait)]
    fn get_attribute_node_ns<'a>(
        &'a self,
        namespace: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, <Self::Attributes<'a> as Attributes<'a>>::Attribute>> {
        self.attributes().get_named_item_ns(namespace, name)
    }

    fn get_attribute<'a>(
        &'a self,
        name: &<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> Option<Self::Atom> {
        self.get_attribute_node(name).map(|a| a.value.clone())
    }

    #[allow(refining_impl_trait)]
    fn get_attribute_local(
        &self,
        name: &<<Self::Attr as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Atom> {
        self.get_attribute_node_local(name).map(|a| a.value.clone())
    }

    #[allow(refining_impl_trait)]
    fn get_attribute_ns<'a>(
        &'a self,
        namespace: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<<Self::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Atom> {
        self.get_attribute_node_ns(namespace, name)
            .map(|a| a.value.clone())
    }

    fn get_attribute_names(
        &self,
    ) -> Vec<<<Self::Attributes<'_> as Attributes<'_>>::Attribute as Attr>::Name> {
        let mut output = vec![];
        let i = 0;
        while let Ok(r) = Ref::filter_map(self.attributes().0.borrow(), |a| a.get(i)) {
            output.push(r.name.clone());
        }
        output
    }

    fn for_each_element_child<F>(&self, mut f: F)
    where
        F: FnMut(Self),
    {
        self.node.0.children.borrow().iter().for_each(|n| {
            if let NodeData::Element { .. } = &n.data {
                f(Self {
                    node: Node5Ever(n.clone()),
                    selector_flags: Cell::new(None),
                });
            }
        });
    }

    fn sort_child_elements<F>(&self, mut f: F)
    where
        F: FnMut(Self, Self) -> std::cmp::Ordering,
    {
        self.node.0.children.borrow_mut().sort_by(|a, b| {
            let Some(a) = Element::new(Node5Ever(a.clone())) else {
                return std::cmp::Ordering::Less;
            };
            let Some(b) = Element::new(Node5Ever(b.clone())) else {
                return std::cmp::Ordering::Greater;
            };
            f(a, b)
        });
    }
}

impl std::hash::Hash for Element5Ever {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.node.0).hash(state);
    }
}

impl Eq for Element5Ever {}

impl PartialEq for Element5Ever {
    fn eq(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.node.0).eq(&Rc::as_ptr(&other.node.0))
    }
}

impl Node for Element5Ever {
    type Atom = StrTendril;
    type Child = Node5Ever;
    type ParentChild = Node5Ever;
    type Parent = Node5Ever;

    fn ptr_eq(&self, other: &impl Node) -> bool {
        self.node.ptr_eq(other)
    }

    fn as_ptr_byte(&self) -> usize {
        self.node.as_ptr_byte()
    }

    // fn as_ref(&self) -> Box<dyn node::Ref> {
    //     self.node.as_ref()
    // }

    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self::Child> {
        self.node.child_nodes().into_iter()
    }

    fn child_node_count(&self) -> usize {
        self.node.child_node_count()
    }

    fn child_nodes(&self) -> Vec<Self::Child> {
        self.node.child_nodes()
    }

    fn for_each_child<F>(&self, f: F)
    where
        F: FnMut(Self::Child),
    {
        self.node.for_each_child(f);
    }

    fn try_for_each_child<F, E>(&self, f: F) -> Result<(), E>
    where
        F: FnMut(Self::Child) -> Result<(), E>,
    {
        self.node.try_for_each_child(f)
    }

    fn any_child<F>(&self, f: F) -> bool
    where
        F: FnMut(Self::Child) -> bool,
    {
        self.node.any_child(f)
    }

    fn all_children<F>(&self, f: F) -> bool
    where
        F: FnMut(Self::Child) -> bool,
    {
        self.node.all_children(f)
    }

    fn retain_children<F>(&self, f: F)
    where
        F: FnMut(Self::Child) -> bool,
    {
        self.node.retain_children(f)
    }

    fn element(&self) -> Option<impl Element> {
        Some(Node::to_owned(self))
    }

    fn empty(&self) {
        self.node.empty();
    }

    fn find_element(&self) -> Option<impl Element> {
        self.element()
    }

    fn node_type(&self) -> node::Type {
        self.node.node_type()
    }

    fn processing_instruction(&self) -> Option<(Self::Atom, Self::Atom)> {
        None
    }

    #[allow(refining_impl_trait)]
    fn parent_node(&self) -> Option<Node5Ever> {
        self.node.parent_node()
    }

    #[allow(refining_impl_trait)]
    fn set_parent_node(&self, new_parent: &Self::Parent) -> Option<Node5Ever> {
        self.node.set_parent_node(&new_parent)
    }

    fn append_child(&mut self, a_child: Self::Child) {
        self.node.append_child(a_child);
    }

    fn insert(&mut self, index: usize, new_node: Self::Child) {
        self.node.insert(index, new_node);
    }

    fn node_name(&self) -> Self::Atom {
        self.node.node_name()
    }

    fn node_value(&self) -> Option<Self::Atom> {
        None
    }

    fn try_set_node_value(&self, _value: Self::Atom) -> Option<()> {
        None
    }

    fn text_content(&self) -> Option<String> {
        self.node.text_content()
    }

    fn set_text_content(&mut self, content: Self::Atom) {
        self.node.set_text_content(content);
    }

    fn text(&self, content: Self::Atom) -> Self::Child {
        self.node.text(content)
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
        Self::new(Node::to_owned(&self.node)).unwrap()
    }

    fn as_impl(&self) -> impl Node {
        self.node.as_impl()
    }

    fn as_child(&self) -> Self::Child {
        self.node.clone()
    }

    fn as_parent_child(&self) -> Self::ParentChild {
        Node::to_owned(&self.node)
    }
}

struct Element5EverData<'a> {
    name: &'a QualName,
    attrs: &'a RefCell<Vec<Attribute>>,
}

impl Element5Ever {
    /// Get's the associated element data.
    fn data(&self) -> Element5EverData {
        if let NodeData::Element { name, attrs, .. } = &self.node.0.data {
            Element5EverData { name, attrs }
        } else {
            log::debug!(
                "You probably tried getting something element related from a document element. Check the stack trace."
            );
            log::debug!("{:?}", self.node);
            unreachable!("Element contains non-element data. This is a bug!")
        }
    }

    #[cfg(feature = "selectors")]
    pub fn set_selector_flags(&self, selector_flags: selectors::matching::ElementSelectorFlags) {
        if selector_flags.is_empty() {
            return;
        };
        self.selector_flags.set(Some(
            selector_flags | self.selector_flags.take().unwrap_or(selector_flags),
        ));
    }
}

impl Debug for Element5Ever {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.node_type() != node::Type::Element {
            return self.node.fmt(f);
        }
        let name = self.qual_name().formatter();
        let attributes = self.attributes();
        let text = self.text_content().map(|s| s.trim().to_string());
        let child_count = match self.node.0.children.borrow().len() {
            0 => String::from("/>"),
            len => format!(">{len} child nodes</{name}>"),
        };
        f.write_fmt(format_args!(
            r"Element5Ever {{ <{name} {attributes:?}{child_count} {text:?} }}"
        ))
    }
}

impl node::Features for Element5Ever {}

#[cfg(feature = "parse")]
impl parse::Node for Element5Ever {
    fn parse(source: &str) -> anyhow::Result<Self> {
        let root = Node5Ever::parse(source)?;
        match Node5Ever::find_element(&root) {
            Some(element) => Ok(element),
            None => Err(anyhow::Error::new(parse::Error::NoElementInDocument)),
        }
    }

    fn parse_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        Self::parse_file(&file)
    }

    fn parse_file(file: &std::fs::File) -> anyhow::Result<Self> {
        let root = Node5Ever::parse_file(file)?;
        match Node5Ever::find_element(&root) {
            Some(element) => Ok(element),
            None => Err(anyhow::Error::new(parse::Error::NoElementInDocument)),
        }
    }
}

impl node::Ref for Node5EverRef {
    fn inner_as_any(&self) -> &dyn std::any::Any {
        let inner: &Node5Ever = self.0.as_ref();
        inner as &dyn std::any::Any
    }

    fn clone(&self) -> Box<dyn node::Ref> {
        Box::new(Self(self.0.clone()))
    }
}

#[cfg(feature = "serialize")]
impl serialize::Node for Element5Ever {
    fn serialize_with_options(&self, options: serialize::Options) -> anyhow::Result<String> {
        self.node.serialize_with_options(options)
    }

    fn serialize(&self) -> anyhow::Result<String> {
        self.node.serialize()
    }

    fn serialize_into<Wr: std::io::Write>(&self, sink: Wr) -> anyhow::Result<()> {
        self.node.serialize_into(sink)
    }
}

impl element::Features for Element5Ever {}

#[cfg(feature = "selectors")]
impl selectors::Element for Element5Ever {
    type Impl = crate::selectors::SelectorImpl<
        <Self as Node>::Atom,
        <<Self as Element>::Name as Name>::LocalName,
        <<Self as Element>::Name as Name>::Namespace,
    >;

    fn opaque(&self) -> selectors::OpaqueElement {
        selectors::OpaqueElement::new(self)
    }

    fn parent_element(&self) -> Option<Self> {
        Element::parent_element(self)
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        Element::previous_element_sibling(self)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        Element::next_element_sibling(self)
    }

    fn first_element_child(&self) -> Option<Self> {
        self.children().first().cloned()
    }

    fn is_html_element_in_html_document(&self) -> bool {
        true
    }

    fn has_local_name(
        &self,
        local_name: &<Self::Impl as selectors::SelectorImpl>::BorrowedLocalName,
    ) -> bool {
        if self.node_type() == node::Type::Document {
            false
        } else {
            self.local_name() == &local_name.0
        }
    }

    fn has_namespace(
        &self,
        ns: &<Self::Impl as selectors::SelectorImpl>::BorrowedNamespaceUrl,
    ) -> bool {
        self.qual_name().ns() == &ns.0
    }

    fn is_same_type(&self, other: &Self) -> bool {
        let name = self.qual_name();
        let other_name = other.qual_name();

        name.local_name() == other_name.local_name() && name.prefix() == other_name.prefix()
    }

    fn attr_matches(
        &self,
        ns: &selectors::attr::NamespaceConstraint<
            &<Self::Impl as selectors::SelectorImpl>::NamespaceUrl,
        >,
        local_name: &<Self::Impl as selectors::SelectorImpl>::LocalName,
        operation: &selectors::attr::AttrSelectorOperation<
            &<Self::Impl as selectors::SelectorImpl>::AttrValue,
        >,
    ) -> bool {
        use selectors::attr::NamespaceConstraint;

        let value = match ns {
            NamespaceConstraint::Any => self.get_attribute_local(&local_name.0),
            NamespaceConstraint::Specific(ns) => self.get_attribute_ns(&ns.0, &local_name.0),
        };
        let Some(value) = value else {
            return false;
        };
        let string = value.as_ref();
        operation.eval_str(string)
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &<Self::Impl as selectors::SelectorImpl>::NonTSPseudoClass,
        _context: &mut selectors::context::MatchingContext<Self::Impl>,
    ) -> bool {
        use crate::selectors::PseudoClass;

        match pc {
            PseudoClass::Link(..) | PseudoClass::AnyLink(..) => self.is_link(),
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &<Self::Impl as selectors::SelectorImpl>::PseudoElement,
        _context: &mut selectors::context::MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, flags: selectors::matching::ElementSelectorFlags) {
        let self_flags = flags.for_self();
        self.set_selector_flags(self_flags);

        let Some(parent) = Element::parent_element(self) else {
            return;
        };
        let parent_flags = flags.for_parent();
        parent.set_selector_flags(parent_flags);
    }

    fn is_link(&self) -> bool {
        if self.node_type() == node::Type::Document {
            return false;
        }
        matches!(
            self.local_name(),
            &local_name!("a") | &local_name!("area") | &local_name!("link")
        ) && self.has_attribute_local(&local_name!("href"))
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        let Some(self_id) = self.get_attribute_local(&local_name!("id")) else {
            return false;
        };
        case_sensitivity.eq(id.0.as_bytes(), self_id.as_bytes())
    }

    fn has_class(
        &self,
        name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        if self.node_type() == node::Type::Document {
            return false;
        }

        let Some(self_class) = self.get_attribute_local(&local_name!("class")) else {
            return false;
        };
        let name = name.0.as_bytes();
        self_class
            .split_whitespace()
            .any(|c| case_sensitivity.eq(name, c.as_bytes()))
    }

    fn imported_part(
        &self,
        _name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
    ) -> Option<<Self::Impl as selectors::SelectorImpl>::Identifier> {
        None
    }

    fn is_part(&self, _name: &<Self::Impl as selectors::SelectorImpl>::Identifier) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        !self.has_child_nodes()
            || self.all_children(|child| match &child.0.data {
                NodeData::Text { contents } => contents.borrow().trim().is_empty(),
                _ => false,
            })
    }

    fn is_root(&self) -> bool {
        let Some(parent) = self.parent_node() else {
            return true;
        };
        parent.node_type() == node::Type::Document
    }

    fn has_custom_state(
        &self,
        _name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
    ) -> bool {
        false
    }

    #[allow(clippy::cast_possible_truncation)]
    fn add_element_unique_hashes(&self, filter: &mut selectors::bloom::BloomFilter) -> bool {
        let mut f = |hash: u32| filter.insert_hash(hash & selectors::bloom::BLOOM_HASH_MASK);

        let local_name_hash = &mut DefaultHasher::default();
        self.local_name().hash(local_name_hash);
        f(local_name_hash.finish() as u32);

        let prefix_hash = &mut DefaultHasher::default();
        self.prefix().hash(prefix_hash);
        f(prefix_hash.finish() as u32);

        if let Some(id) = self.get_attribute(&QualName {
            prefix: None,
            ns: Namespace::default(),
            local: local_name!("id"),
        }) {
            let id_hash = &mut DefaultHasher::default();
            id.hash(id_hash);
            f(prefix_hash.finish() as u32);
        }

        for class in self.class_list().iter() {
            let class_hash = &mut DefaultHasher::default();
            class.hash(class_hash);
            f(class_hash.finish() as u32);
        }

        for attr in self.attributes().into_iter() {
            let name = attr.name();
            if matches!(name.local_name().as_ref(), "class" | "id" | "style") {
                continue;
            }

            let name_hash = &mut DefaultHasher::default();
            name.hash(name_hash);
            f(name_hash.finish() as u32);
        }
        true
    }
}

impl Document for Document5Ever {
    type Root = Element5Ever;

    fn document_element(&self) -> &Self::Root {
        &self.0
    }

    fn create_attribute<'a>(
        &self,
        name: <<<Self::Root as Element>::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> <<Self::Root as Element>::Attributes<'a> as Attributes<'a>>::Attribute {
        Attribute {
            name,
            value: StrTendril::default(),
        }
    }

    fn create_c_data_section(&self, data: <Self::Root as Node>::Atom) -> Node5Ever {
        Self::create_node(rcdom::NodeData::Text {
            contents: RefCell::new(data),
        })
    }

    fn create_element(&self, tag_name: <Self::Root as Element>::Name) -> Self::Root {
        Element5Ever {
            node: Self::create_node(rcdom::NodeData::Element {
                name: tag_name,
                attrs: RefCell::new(vec![]),
                template_contents: RefCell::new(None),
                mathml_annotation_xml_integration_point: false,
            }),
            selector_flags: Cell::new(None),
        }
    }

    fn create_processing_instruction(
        &self,
        target: <Self::Root as Node>::Atom,
        data: <Self::Root as Node>::Atom,
    ) -> <Self::Root as Node>::Child {
        Self::create_node(rcdom::NodeData::ProcessingInstruction {
            target,
            contents: data,
        })
    }

    fn create_text_node(&self, data: <Self::Root as Node>::Atom) -> <Self::Root as Node>::Child {
        Self::create_node(rcdom::NodeData::Text {
            contents: RefCell::new(data),
        })
    }
}

impl Document5Ever {
    fn create_node(data: rcdom::NodeData) -> Node5Ever {
        Node5Ever(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            children: RefCell::new(vec![]),
            data,
        }))
    }
}
