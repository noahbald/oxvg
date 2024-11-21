use core::panic;
use std::{
    cell::{Cell, RefCell, RefMut},
    collections::VecDeque,
    fmt::Debug,
    rc::Rc,
};

use markup5ever::{
    local_name, tendril::StrTendril, Attribute, LocalName, Namespace, NamespaceStaticSet, Prefix,
    QualName,
};
use rcdom::NodeData;

use crate::{
    attribute::{Attr, Attributes},
    element::{self, Element},
    name::Name,
    node::{self, Node},
};

#[cfg(feature = "parse")]
use crate::parse;

#[cfg(feature = "serialize")]
use crate::serialize;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Atom5Ever(StrTendril);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Prefix5Ever(Prefix);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct LocalName5Ever(LocalName);

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Namespace5Ever(string_cache::Atom<NamespaceStaticSet>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct QualName5Ever(QualName);

enum Attribute5Ever<'a> {
    Borrowed(RefMut<'a, Attribute>),
    Owned(Attribute),
}

struct Attributes5Ever<'a>(&'a RefCell<Vec<Attribute>>);

#[derive(Clone)]
struct Node5Ever(Rc<rcdom::Node>);

#[derive(Clone)]
struct Element5Ever {
    node: Node5Ever,
    #[cfg(feature = "selectors")]
    selector_flags: Cell<Option<selectors::matching::ElementSelectorFlags>>,
}

impl crate::atom::Atom for Atom5Ever {}

impl From<&str> for Atom5Ever {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<Atom5Ever> for String {
    fn from(val: Atom5Ever) -> Self {
        val.0.to_string()
    }
}

impl From<String> for Atom5Ever {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl Into<String> for &Atom5Ever {
    fn into(self) -> String {
        self.0.to_string()
    }
}

impl AsRef<str> for Atom5Ever {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl crate::atom::Atom for LocalName5Ever {}

impl From<&str> for LocalName5Ever {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<LocalName5Ever> for String {
    fn from(val: LocalName5Ever) -> Self {
        val.0.to_string()
    }
}

impl From<String> for LocalName5Ever {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for LocalName5Ever {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl crate::atom::Atom for Prefix5Ever {}

impl From<&str> for Prefix5Ever {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<Prefix5Ever> for String {
    fn from(val: Prefix5Ever) -> Self {
        val.0.to_string()
    }
}

impl From<String> for Prefix5Ever {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for Prefix5Ever {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl crate::atom::Atom for Namespace5Ever {}

impl From<&str> for Namespace5Ever {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<Namespace5Ever> for String {
    fn from(value: Namespace5Ever) -> Self {
        value.0.to_string()
    }
}

impl From<String> for Namespace5Ever {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for Namespace5Ever {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

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
        match value.split_once(':') {
            Some((prefix, local)) => Self(QualName::new(
                Some(prefix.into()),
                Namespace::default(),
                local.into(),
            )),
            None => Self(QualName::new(None, Namespace::default(), value.into())),
        }
    }
}

impl Default for QualName5Ever {
    fn default() -> Self {
        Self(QualName::new(None, Namespace::default(), "".into()))
    }
}

impl Attr<'_> for Attribute5Ever<'_> {
    type Atom = Atom5Ever;
    type Name = QualName5Ever;

    fn value(&self) -> Self::Atom {
        Atom5Ever(self.inner().value.clone())
    }

    fn name(&self) -> Self::Name {
        QualName5Ever(self.inner().name.clone())
    }
}

impl Attribute5Ever<'_> {
    fn inner(&self) -> &Attribute {
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

impl<'a> Attributes<'a> for Attributes5Ever<'a> {
    type Attribute<'b> = Attribute5Ever<'b> where 'a: 'b;

    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn item(&self, index: usize) -> Option<Self::Attribute<'a>> {
        let attr = RefMut::filter_map(self.0.borrow_mut(), |v| v.get_mut(index)).ok()?;
        Some(Attribute5Ever::Borrowed(attr))
    }

    fn get_named_item(&self, name: &LocalName5Ever) -> Option<Self::Attribute<'a>> {
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
    ) -> Option<Self::Attribute<'a>> {
        let attr = RefMut::filter_map(self.0.borrow_mut(), |v| {
            v.iter_mut()
                .find(|a| a.name.local == name.0 && a.name.ns == namespace.0)
        })
        .ok()?;
        Some(Attribute5Ever::Borrowed(attr))
    }

    fn remove_named_item(
        &self,
        name: &<Self::Attribute<'a> as Attr<'a>>::Name,
    ) -> Option<Self::Attribute<'a>> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs.iter().position(|a| a.name == name.0)?;
        Some(Attribute5Ever::Owned(attrs.remove(index)))
    }

    fn set_named_item(&self, attr: Self::Attribute<'a>) -> Option<Self::Attribute<'a>> {
        let Attribute5Ever::Owned(attr) = attr else {
            panic!("Tried setting attribute to borrowed value, try cloning first");
        };
        let attrs = &mut *self.0.borrow_mut();
        let index = attrs.iter().position(|a| a.name == attr.name)?;
        let old_attr = std::mem::replace(&mut attrs[index], attr);
        Some(Attribute5Ever::Owned(old_attr))
    }

    fn iter(&'a self) -> impl Iterator<Item = Self::Attribute<'a>> {
        AttributesIterator {
            index: 0,
            attrs_ref: self.0,
        }
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

impl Node for Node5Ever {
    type Atom = Atom5Ever;
    type Child = Node5Ever;
    type ParentChild = Node5Ever;

    fn ptr_eq(&self, other: &impl Node) -> bool {
        let other: &dyn std::any::Any = other;
        let Some(downcast) = other.downcast_ref::<&Self>() else {
            return false;
        };
        Rc::ptr_eq(&downcast.0, &self.0)
    }

    fn child_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Self> {
        let children = self.0.children.borrow().clone();
        children.into_iter().map(Self)
    }

    fn child_nodes(&self) -> Vec<Self::Child> {
        self.child_nodes_iter().collect()
    }

    fn element(&self) -> Option<Element5Ever> {
        match self.node_type() {
            node::Type::Element => Element5Ever::new(Node::to_owned(self)),
            _ => None,
        }
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

    fn parent_node(&self) -> Option<impl Node<Child = Self, Atom = Self::Atom>> {
        let cell = &self.0.parent;
        let parent = cell.take()?;
        let node = parent.upgrade().map(Self);
        cell.set(Some(parent));
        node
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

    fn clone_node(&self) -> Self {
        let data = match &self.0.data {
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
        };
        let children = self.child_nodes_iter().map(|c| c.clone_node().0).collect();
        Self(Rc::new(rcdom::Node {
            parent: Cell::new(None),
            data,
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
        use rcdom::SerializableHandle;
        use xml5ever::serialize::{serialize, SerializeOpts};

        let mut sink: std::io::BufWriter<_> = std::io::BufWriter::new(Vec::new());
        serialize(
            &mut sink,
            &std::convert::Into::<SerializableHandle>::into(self.0.clone()),
            SerializeOpts::default(),
        )?;

        let sink: Vec<_> = sink.into_inner()?;
        Ok(String::from_utf8_lossy(&sink).to_string())
    }
}

impl Element for Element5Ever {
    type Name = QualName5Ever;
    type Attributes<'a> = Attributes5Ever<'a>;

    fn new(node: Node5Ever) -> Option<Self> {
        if node.node_type() != node::Type::Element {
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

    fn local_name(&self) -> LocalName5Ever {
        self.qual_name().local_name()
    }

    fn attributes(&self) -> Self::Attributes<'_> {
        Attributes5Ever(self.data().attrs)
    }

    fn remove(&self) {
        let Some(mut parent) = self.parent_node() else {
            // Element already removed
            return;
        };

        parent.remove_child(self.as_node());
        self.node.0.parent.set(None);
    }

    fn prefix(&self) -> Option<Prefix5Ever> {
        self.qual_name().prefix()
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
}

impl Node for Element5Ever {
    type Atom = Atom5Ever;
    type Child = Node5Ever;
    type ParentChild = Node5Ever;

    fn ptr_eq(&self, other: &impl Node) -> bool {
        self.node.ptr_eq(other)
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

    fn node_type(&self) -> node::Type {
        self.node.node_type()
    }

    fn parent_node(&self) -> Option<impl Node<Child = Self::ParentChild, Atom = Self::Atom>> {
        self.node.parent_node()
    }

    fn node_name(&self) -> Self::Atom {
        self.node.node_name()
    }

    fn node_value(&self) -> Option<Self::Atom> {
        self.node.node_value()
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
    fn qual_name(&self) -> <Self as Element>::Name {
        QualName5Ever(self.data().name.clone())
    }

    fn data(&self) -> Element5EverData {
        let NodeData::Element { name, attrs, .. } = &self.node.0.data else {
            unreachable!("Element contains non-element data. This is a bug!")
        };
        Element5EverData { name, attrs }
    }

    fn as_node(&self) -> <Self as Node>::Child {
        self.node.clone()
    }

    fn find_element(node: Node5Ever) -> Option<Self> {
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
        f.write_fmt(format_args!(
            r"Element5Ever {{
    data: {:?},
}}",
            self.node.0.data
        ))
    }
}

#[cfg(feature = "parse")]
impl parse::Node for Element5Ever {
    fn parse(source: &str) -> anyhow::Result<Self> {
        let root = Node5Ever::parse(source)?;
        match Self::find_element(root) {
            Some(element) => Ok(element),
            None => Err(anyhow::Error::new(parse::Error::NoElementInDocument)),
        }
    }
}

#[cfg(feature = "serialize")]
impl serialize::Node for Element5Ever {
    fn serialize(&self) -> anyhow::Result<String> {
        self.node.serialize()
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
        self.children().first().map(Element5Ever::clone)
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
            NamespaceConstraint::Any => self.get_attribute(&local_name.0),
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
        ) && self.has_attribute(&LocalName5Ever(local_name!("href")))
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        let Some(self_id) = self.get_attribute(&LocalName5Ever(local_name!("id"))) else {
            return false;
        };
        case_sensitivity.eq(id.0 .0.as_bytes(), self_id.0.as_bytes())
    }

    fn has_class(
        &self,
        name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        let Some(self_class) = self.get_attribute(&LocalName5Ever(local_name!("class"))) else {
            return false;
        };
        case_sensitivity.eq(name.0 .0.as_bytes(), self_class.0.as_bytes())
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
