use core::panic;
use std::{
    cell::{Cell, RefCell, RefMut},
    collections::VecDeque,
    fmt::{Debug, Display},
    rc::Rc,
};

use markup5ever::{
    local_name, tendril::StrTendril, Attribute, LocalName, Namespace, NamespaceStaticSet, Prefix,
    QualName,
};
use rcdom::NodeData;

use crate::{
    attribute::{Attr, Attributes},
    class_list::ClassList,
    element::{self, Element},
    name::Name,
    node::{self, Node, Ref},
};

#[cfg(feature = "parse")]
use crate::parse;

#[cfg(feature = "serialize")]
use crate::serialize;

macro_rules! atom {
    ($name:ident) => {
        impl crate::atom::Atom for $name {}

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.into())
            }
        }

        impl From<$name> for String {
            fn from(val: $name) -> Self {
                val.0.to_string()
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }

        impl From<&$name> for String {
            fn from(val: &$name) -> Self {
                val.0.to_string()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.0.as_ref()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.0, f)
            }
        }
    };
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Atom5Ever(StrTendril);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Prefix5Ever(Prefix);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct LocalName5Ever(LocalName);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Namespace5Ever(string_cache::Atom<NamespaceStaticSet>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualName5Ever(QualName);

#[derive(Debug)]
pub enum Attribute5Ever<'a> {
    Borrowed(RefMut<'a, Attribute>),
    Owned(Attribute),
}

#[derive(Clone)]
pub struct Attributes5Ever<'a>(&'a RefCell<Vec<Attribute>>);

pub struct ClassList5Ever<'a> {
    attrs: Attributes5Ever<'a>,
    class_index_memo: Cell<usize>,
    tokens: Vec<Atom5Ever>,
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

atom!(Atom5Ever);
atom!(LocalName5Ever);
atom!(Prefix5Ever);
atom!(Namespace5Ever);

impl Name for QualName5Ever {
    type LocalName = LocalName5Ever;
    type Prefix = Prefix5Ever;
    type Namespace = Namespace5Ever;

    fn local_name(&self) -> Self::LocalName {
        LocalName5Ever(self.0.local.clone())
    }

    fn prefix(&self) -> Option<Self::Prefix> {
        Some(Prefix5Ever(self.0.prefix.clone()?))
    }

    fn ns(&self) -> Self::Namespace {
        Namespace5Ever(self.0.ns.clone())
    }
}

impl From<&str> for QualName5Ever {
    fn from(value: &str) -> Self {
        let mut parts = value.split(':');
        let prefix_or_local = parts
            .next()
            .expect("Attempted to make qual-name from empty string");
        let maybe_local = parts.next();
        assert_eq!(parts.next(), None);

        match maybe_local {
            Some(local) => Self(QualName {
                prefix: Some(prefix_or_local.into()),
                local: local.into(),
                ns: string_cache::Atom::default(),
            }),
            None => Self(QualName {
                prefix: None,
                local: prefix_or_local.into(),
                ns: string_cache::Atom::default(),
            }),
        }
    }
}

impl Default for QualName5Ever {
    fn default() -> Self {
        Self(QualName::new(None, Namespace::default(), "".into()))
    }
}

impl From<String> for QualName5Ever {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl Display for QualName5Ever {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let local = &self.0.local;
        match &self.0.prefix {
            Some(prefix) => f.write_fmt(format_args!("{prefix}:{local}")),
            None => Display::fmt(&local, f),
        }
    }
}

impl Attr for Attribute5Ever<'_> {
    type Atom = Atom5Ever;
    type Name = QualName5Ever;

    fn value(&self) -> Self::Atom {
        Atom5Ever(self.inner().value.clone())
    }

    fn value_ref(&self) -> &str {
        self.inner().value.as_ref()
    }

    fn set_value(&mut self, value: Self::Atom) -> Self::Atom {
        Atom5Ever(std::mem::replace(&mut self.inner_mut().value, value.0))
    }

    fn name(&self) -> Self::Name {
        QualName5Ever(self.inner().name.clone())
    }

    fn into_owned(self) -> Self {
        match self {
            Self::Owned(_) => self,
            Self::Borrowed(attr) => Self::Owned(attr.clone()),
        }
    }
}

impl Attribute5Ever<'_> {
    /// Returns the associated attribute
    fn inner(&self) -> &Attribute {
        match self {
            Self::Owned(attr) => attr,
            Self::Borrowed(attr) => attr,
        }
    }

    /// Mutable returns the associated attribute
    fn inner_mut(&mut self) -> &mut Attribute {
        match self {
            Self::Owned(attr) => attr,
            Self::Borrowed(attr) => attr,
        }
    }
}

impl PartialEq for Attribute5Ever<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.inner() == other.inner()
    }
}

impl From<(QualName5Ever, Atom5Ever)> for Attribute5Ever<'_> {
    fn from(value: (QualName5Ever, Atom5Ever)) -> Self {
        let (QualName5Ever(name), Atom5Ever(value)) = value;
        Self::Owned(Attribute { name, value })
    }
}

impl From<(LocalName5Ever, Atom5Ever)> for Attribute5Ever<'_> {
    fn from(value: (LocalName5Ever, Atom5Ever)) -> Self {
        let (LocalName5Ever(name), Atom5Ever(value)) = value;
        Self::Owned(Attribute {
            name: QualName {
                local: name,
                prefix: None,
                ns: string_cache::Atom::default(),
            },
            value,
        })
    }
}

impl Display for Attribute5Ever<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name();
        let value = self.value();
        f.write_fmt(format_args!(r#"{name}="{value}""#))
    }
}

impl<'a> Attributes<'a> for Attributes5Ever<'a> {
    type Attribute = Attribute5Ever<'a>;

    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn item(&self, index: usize) -> Option<Self::Attribute> {
        let attr = RefMut::filter_map(self.0.borrow_mut(), |v| v.get_mut(index)).ok()?;
        Some(Attribute5Ever::Borrowed(attr))
    }

    fn get_named_item(&self, name: &QualName5Ever) -> Option<Self::Attribute> {
        let attr = RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut()
                .find(|a| a.name.prefix == name.0.prefix && a.name.local == name.0.local)
        })
        .ok()?;
        Some(Attribute5Ever::Borrowed(attr))
    }

    fn get_named_item_local(&self, name: &LocalName5Ever) -> Option<Self::Attribute> {
        let attr = RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut().find(|a| a.name.local == name.0)
        })
        .ok()?;
        Some(Attribute5Ever::Borrowed(attr))
    }

    fn get_named_item_ns(
        &self,
        namespace: &Namespace5Ever,
        name: &LocalName5Ever,
    ) -> Option<Self::Attribute> {
        let attr = RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut()
                .find(|a| a.name.local == name.0 && a.name.ns == namespace.0)
        })
        .ok()?;
        Some(Attribute5Ever::Borrowed(attr))
    }

    fn remove_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Attribute> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs
            .iter()
            .position(|a| a.name.prefix == name.0.prefix && a.name.local == name.0.local)?;
        Some(Attribute5Ever::Owned(attrs.remove(index)))
    }

    fn remove_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs.iter().position(|a| a.name.local == name.0)?;
        Some(Attribute5Ever::Owned(attrs.remove(index)))
    }

    fn set_named_item(&self, attr: Self::Attribute) -> Option<Self::Attribute> {
        let Attribute5Ever::Owned(attr) = attr else {
            panic!("Tried setting attribute to borrowed value, try cloning first");
        };
        let attrs = &mut *self.0.borrow_mut();
        if let Some(index) = attrs
            .iter()
            .position(|a| a.name.prefix == attr.name.prefix && a.name.local == attr.name.local)
        {
            Some(Attribute5Ever::Owned(std::mem::replace(
                &mut attrs[index],
                attr,
            )))
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
        let attr = Attribute5Ever::Owned(Attribute {
            name: name.0,
            value: value.0,
        });
        self.set_named_item(attr)
    }

    fn iter(&self) -> impl Iterator<Item = Self::Attribute> {
        AttributesIterator {
            index: 0,
            attrs_ref: self.0,
        }
    }
}

impl Debug for Attributes5Ever<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Attribute5Ever { ")?;
        self.iter()
            .try_for_each(|a| f.write_fmt(format_args!(r#"{}="{}" "#, a.name(), a.value())))?;
        f.write_str("} ")
    }
}

struct AttributesIterator<'a> {
    index: usize,
    attrs_ref: &'a RefCell<Vec<Attribute>>,
}

impl<'a> Iterator for AttributesIterator<'a> {
    type Item = Attribute5Ever<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let result =
            RefMut::filter_map(self.attrs_ref.borrow_mut(), |v| v.get_mut(self.index)).ok()?;
        let result = Attribute5Ever::Borrowed(result);
        self.index += 1;
        Some(result)
    }
}

impl<'a> ClassList for ClassList5Ever<'a> {
    type Attribute = Attribute5Ever<'a>;

    fn length(&self) -> usize {
        self.tokens.len()
    }

    fn value(&self) -> <Self::Attribute as Attr>::Atom {
        self.attr()
            .map(|a| Atom5Ever(a.value.clone()))
            .unwrap_or_default()
    }

    fn add(&mut self, token: <Self::Attribute as Attr>::Atom) {
        if self.contains(&token) {
            return;
        };
        let Some(mut attr) = self.attr() else {
            self.attrs.set_named_item(Attribute5Ever::Owned(Attribute {
                name: QualName {
                    prefix: None,
                    local: local_name!("class"),
                    ns: Namespace::default(),
                },
                value: token.0.clone(),
            }));
            self.tokens.push(token);
            return;
        };

        attr.value.push_tendril(&token.0);
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
        let mut new_value = attr.value.subtendril(0, start as u32);
        new_value.push_tendril(&attr.value.subtendril(end, attr.value.len() as u32 - end));
        drop(attr);
        if new_value.trim().is_empty() {
            self.attrs
                .remove_named_item_local(&LocalName5Ever(local_name!("class")));
        } else {
            self.attrs.set_named_item_qual(
                QualName5Ever(QualName {
                    prefix: None,
                    local: local_name!("class"),
                    ns: Namespace::default(),
                }),
                Atom5Ever(new_value),
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

        let token_tendril = new_token.0.clone();
        self.tokens[index] = new_token;
        let attr = self.attr().expect("had token");
        let mut new_value = attr.value.subtendril(0, start);
        new_value.push_tendril(&token_tendril);
        new_value.push_tendril(&attr.value.subtendril(end, attr.value.len() as u32 - end));

        self.attrs.set_named_item_qual(
            QualName5Ever(QualName {
                prefix: None,
                local: local_name!("class"),
                ns: Namespace::default(),
            }),
            Atom5Ever(new_value),
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
                if end - start == token.0.len() {
                    break;
                }
                saw_whitespace = true;
                continue;
            }
            if skip_to_next_word {
                continue;
            }
            if token.0.chars().nth(end - start).is_some_and(|c| c == char) {
                end = i + 1;
                continue;
            }
            skip_to_next_word = true;
        }
        if end - start < token.0.len() || skip_to_next_word {
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
    type Atom = Atom5Ever;
    type Child = Node5Ever;
    type ParentChild = Node5Ever;

    fn ptr_eq(&self, other: &impl Node) -> bool {
        self.as_ptr_byte() == other.as_ptr_byte()
    }

    fn as_ptr_byte(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    fn as_ref(&self) -> Box<dyn node::Ref> {
        Box::new(Node5EverRef(Rc::new(self.clone())))
    }

    fn c_data(&self, contents: Self::Atom) -> Self::Child {
        Node5Ever(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            children: RefCell::new(vec![]),
            // WARN: rcdom doesn't support CData
            data: NodeData::Text {
                contents: RefCell::new(contents.0),
            },
        }))
    }

    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        let children = self.0.children.borrow().clone();
        children.into_iter().map(Self)
    }

    fn child_nodes(&self) -> Vec<Self::Child> {
        self.child_nodes_iter().collect()
    }

    #[allow(refining_impl_trait)]
    fn element(&self) -> Option<Element5Ever> {
        match self.node_type() {
            node::Type::Element => Element5Ever::new(Node::to_owned(self)),
            _ => None,
        }
    }

    #[allow(refining_impl_trait)]
    fn find_element(&self) -> Option<Element5Ever> {
        <Element5Ever as Element>::find_element(Node::to_owned(self))
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
    fn set_parent_node(&self, new_parent: &impl Node<Atom = Self::Atom>) -> Option<Node5Ever> {
        let parent = new_parent as &dyn std::any::Any;
        let parent = parent
            .downcast_ref::<Node5Ever>()
            .expect("Incorrect implementation passed as new parent");
        let parent = Rc::downgrade(&parent.0);
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
            NodeData::Doctype { name, .. } => Atom5Ever(name.clone()),
            NodeData::Document => "#document".into(),
            NodeData::Element { name, .. } => name.local.to_uppercase().into(),
            NodeData::ProcessingInstruction { target, .. } => Atom5Ever(target.clone()),
            NodeData::Text { .. } => "#text".into(),
        }
    }

    fn node_value(&self) -> Option<Self::Atom> {
        Some(match &self.0.data {
            NodeData::Comment { contents } | NodeData::ProcessingInstruction { contents, .. } => {
                Atom5Ever(contents.clone())
            }
            NodeData::Text { contents } => Atom5Ever(contents.borrow().clone()),
            _ => return None,
        })
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

    fn text(&self, content: Self::Atom) -> Self {
        Node5Ever(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            children: RefCell::new(vec![]),
            data: NodeData::Text {
                contents: RefCell::new(content.0),
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
        let children = self.child_nodes_iter().map(|c| c.clone_node().0).collect();
        Self(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            data: self.clone_node_data(),
            children: RefCell::new(children),
        }))
    }

    fn to_owned(&self) -> Self {
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
}

#[cfg(feature = "serialize")]
impl serialize::Node for Node5Ever {
    fn serialize(&self) -> anyhow::Result<String> {
        let mut sink: std::io::BufWriter<_> = std::io::BufWriter::new(Vec::new());
        self.serialize_into(&mut sink)?;

        let sink: Vec<_> = sink.into_inner()?;
        Ok(String::from_utf8_lossy(&sink).to_string())
    }

    fn serialize_into<Wr: std::io::Write>(&self, sink: Wr) -> anyhow::Result<()> {
        use rcdom::SerializableHandle;
        use xml5ever::serialize::{serialize, SerializeOpts};

        Ok(serialize(
            sink,
            &std::convert::Into::<SerializableHandle>::into(self.0.clone()),
            SerializeOpts::default(),
        )?)
    }
}

impl Element for Element5Ever {
    type Name = QualName5Ever;
    type Attributes<'a> = Attributes5Ever<'a>;

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

    fn tag_name(&self) -> Self::Atom {
        self.node.node_name()
    }

    fn qual_name(&self) -> Self::Name {
        QualName5Ever(self.data().name.clone())
    }

    fn set_local_name(&mut self, new_name: <Self::Name as Name>::LocalName) {
        let mut data = self.node.clone_node_data();
        if let rcdom::NodeData::Element { name, .. } = &mut data {
            name.local = new_name.0;
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
        let parent: &dyn std::any::Any = &self.parent_node()?;
        let downcast = parent
            .downcast_ref::<Node5Ever>()
            .expect("Parent node of element should be a node type")
            .clone();
        match downcast.node_type() {
            node::Type::Element => Self::new(downcast),
            _ => None,
        }
    }

    #[allow(refining_impl_trait)]
    fn class_list(&self) -> ClassList5Ever {
        let attrs = self.attributes();
        let attr = attrs.get_named_item_local(&LocalName5Ever(local_name!("class")));
        let tokens = attr
            .as_ref()
            .map(|a| a.value().0.split_whitespace().map(Into::into).collect())
            .unwrap_or_default();
        ClassList5Ever {
            attrs,
            class_index_memo: Cell::new(0),
            tokens,
        }
    }

    fn has_class(&self, token: &Self::Atom) -> bool {
        let token = Atom5Ever(token.0.trim_start_matches('.').into());
        self.class_list().contains(&token)
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
        for child in self.child_nodes_iter() {
            self.after(child);
        }
        self.remove();
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
}

impl Node for Element5Ever {
    type Atom = Atom5Ever;
    type Child = Node5Ever;
    type ParentChild = Node5Ever;

    fn ptr_eq(&self, other: &impl Node) -> bool {
        self.node.ptr_eq(other)
    }

    fn as_ptr_byte(&self) -> usize {
        self.node.as_ptr_byte()
    }

    fn as_ref(&self) -> Box<dyn Ref> {
        self.node.as_ref()
    }

    fn c_data(&self, contents: Self::Atom) -> Self::Child {
        self.node.c_data(contents)
    }

    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self::Child> {
        self.node.child_nodes_iter()
    }

    fn child_nodes(&self) -> Vec<Self::Child> {
        self.node.child_nodes()
    }

    fn element(&self) -> Option<impl Element> {
        Some(Node::to_owned(self))
    }

    fn find_element(&self) -> Option<impl Element> {
        self.element()
    }

    fn node_type(&self) -> node::Type {
        self.node.node_type()
    }

    #[allow(refining_impl_trait)]
    fn parent_node(&self) -> Option<Node5Ever> {
        self.node.parent_node()
    }

    #[allow(refining_impl_trait)]
    fn set_parent_node(&self, new_parent: &impl Node<Atom = Self::Atom>) -> Option<Element5Ever> {
        let new_parent_element = new_parent as &dyn std::any::Any;
        let new_parent_element = new_parent_element.downcast_ref::<Element5Ever>().unwrap();
        let old_parent = Element5Ever {
            node: self.node.set_parent_node(&new_parent_element.node)?,
            selector_flags: Cell::new(None),
        };
        Some(old_parent)
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
        self.node.node_value()
    }

    fn text_content(&self) -> Option<String> {
        self.node.text_content()
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

    fn to_owned(&self) -> Self {
        Self::new(Node::to_owned(&self.node)).unwrap()
    }

    fn as_impl(&self) -> impl Node {
        self.node.as_impl()
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
        let NodeData::Element { name, attrs, .. } = &self.node.0.data else {
            unreachable!("Element contains non-element data. This is a bug!")
        };
        Element5EverData { name, attrs }
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
        let name = self.qual_name();
        let attributes = self.attributes();
        let child_count = match self.node.0.children.borrow().len() {
            0 => String::from("/>"),
            len => format!(">{len} child nodes</{name}>"),
        };
        f.write_fmt(format_args!(
            r"Element5Ever {{ <{name} {attributes:?}{child_count} }}"
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
}

impl Ref for Node5EverRef {
    fn inner_as_any(&self) -> &dyn std::any::Any {
        let inner: &Node5Ever = self.0.as_ref();
        inner as &dyn std::any::Any
    }

    fn clone(&self) -> Box<dyn Ref> {
        Box::new(Self(self.0.clone()))
    }
}

#[cfg(feature = "serialize")]
impl serialize::Node for Element5Ever {
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
        self.local_name() == local_name.0
    }

    fn has_namespace(
        &self,
        ns: &<Self::Impl as selectors::SelectorImpl>::BorrowedNamespaceUrl,
    ) -> bool {
        self.qual_name().ns() == ns.0
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
        let string = value.0.as_ref();
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
        matches!(
            self.local_name().0,
            local_name!("a") | local_name!("area") | local_name!("link")
        ) && self.has_attribute_local(&LocalName5Ever(local_name!("href")))
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        let Some(self_id) = self.get_attribute_local(&LocalName5Ever(local_name!("id"))) else {
            return false;
        };
        case_sensitivity.eq(id.0 .0.as_bytes(), self_id.0.as_bytes())
    }

    fn has_class(
        &self,
        name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        let Some(self_class) = self.get_attribute_local(&LocalName5Ever(local_name!("class")))
        else {
            return false;
        };
        let name = name.0 .0.as_bytes();
        self_class
            .0
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
        self.child_nodes_iter()
            .all(|child| match child.node_type() {
                node::Type::Element => false,
                node::Type::Text => child.node_value().is_some(),
                _ => true,
            })
    }

    fn is_root(&self) -> bool {
        let Some(parent) = self.parent_node() else {
            return true;
        };
        parent.node_type() == node::Type::Document
    }
}
