//! Parsing methods using xml5ever
//!
//! # Quirks
//!
//! xml5ever has some notable quirks
//!
//! - Unused namespaces are skipped
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

use xml5ever::{
    driver::{parse_document, XmlParseOpts},
    interface::{NodeOrText, QuirksMode, TreeSink},
    local_name, namespace_prefix, namespace_url, ns,
    tendril::TendrilSink,
    tree_builder::{ElemName, NamespaceMap},
};

use crate::{
    attribute::Attributes, element::Element as _, implementations::shared::Element, node::Node as _,
};

use super::shared::{Arena, Attribute, Node, NodeData, QualName, Ref};

/// parse an xml file using xml5ever as the parser.
///
/// # Errors
///
/// If the file cannot be read or parsed
pub fn parse_file<'arena>(
    source: &std::path::Path,
    arena: Arena<'arena>,
) -> Result<Ref<'arena>, std::io::Error> {
    parse_document(Sink::new(arena), XmlParseOpts::default())
        .from_utf8()
        .from_file(source)
}

/// parse an xml document using xml5ever as the parser.
pub fn parse<'arena>(source: &str, arena: Arena<'arena>) -> Ref<'arena> {
    parse_document(Sink::new(arena), XmlParseOpts::default()).one(source)
}

struct Allocator<'arena> {
    arena: Arena<'arena>,
    current_node_id: Cell<usize>,
}

impl<'arena> Allocator<'arena> {
    fn alloc(&self, node_data: NodeData) -> &'arena mut Node<'arena> {
        self.current_node_id.set(self.current_node_id.get() + 1);
        self.arena
            .alloc(Node::new(node_data, self.current_node_id.get()))
    }
}

struct Sink<'arena> {
    allocator: Allocator<'arena>,
    document: Ref<'arena>,
    namespace_map: RefCell<NamespaceMap>,
    mode: Cell<QuirksMode>,
    line: Cell<u64>,
}

impl<'arena> Sink<'arena> {
    fn new(arena: Arena<'arena>) -> Self {
        Self {
            allocator: Allocator {
                arena,
                current_node_id: Cell::new(arena.len()),
            },
            document: arena.alloc(Node::new(NodeData::Document, arena.len())),
            namespace_map: RefCell::new(NamespaceMap::empty()),
            mode: Cell::new(QuirksMode::NoQuirks),
            line: Cell::new(1),
        }
    }

    /// Checks whether a namespace is already in the document
    fn find_xml_uri(&self, name: &QualName) -> bool {
        if let Some(Some(el)) = self.namespace_map.borrow().get(&name.prefix) {
            *el == name.ns
        } else {
            false
        }
    }

    /// If a new namespace is found, add it to namespace map and return attribute to add to element
    fn find_new_xmlns(&self, name: &QualName) -> Option<Attribute> {
        if (name.prefix.is_some() || !name.ns.is_empty()) && !self.find_xml_uri(name) {
            self.namespace_map.borrow_mut().insert(&xml5ever::QualName {
                prefix: name.prefix.clone(),
                ns: name.ns.clone(),
                local: name.local.clone(),
            });
            Some(Attribute {
                name: QualName {
                    local: if let Some(prefix) = &name.prefix {
                        prefix.as_ref().into()
                    } else {
                        local_name!("xmlns")
                    },
                    prefix: name.prefix.as_ref().map(|_| namespace_prefix!("xmlns")),
                    ns: ns!(xml),
                },
                value: name.ns.as_ref().into(),
            })
        } else {
            None
        }
    }

    /// Adds `xmlns` attributes to the current element and `xmlns`-prefixed attributes to the root
    fn add_xmlns(&self, child: &<Self as TreeSink>::Handle) {
        let NodeData::Element { name, attrs, .. } = &child.node_data else {
            return;
        };

        let root: Element = Element::new(child)
            .unwrap()
            .document()
            .and_then(|n| n.find_element())
            .unwrap_or_else(|| Element::new(child).unwrap());
        if let Some(attr_to_insert) = self.find_new_xmlns(name) {
            if attr_to_insert.name.prefix.is_none() {
                attrs.borrow_mut().insert(0, attr_to_insert);
            } else {
                root.attributes().set_named_item(attr_to_insert);
            }
        };

        let attrs_ref = attrs.borrow();
        let root_attrs_to_insert: Vec<_> = attrs_ref
            .iter()
            .filter_map(|attr| self.find_new_xmlns(&attr.name))
            .collect();
        drop(attrs_ref);

        let mut root_attrs = root.attributes().0.borrow_mut();
        let has_xmlns = root_attrs.first().is_some_and(|attr| {
            attr.name.prefix.is_none() && attr.name.local == local_name!("xmlns")
        });
        for (i, attr) in root_attrs_to_insert.into_iter().enumerate() {
            if attr.name.local.as_ref() == "xml" {
                continue;
            }
            root_attrs.insert(if has_xmlns { i + 1 } else { i }, attr);
        }
    }
}

impl ElemName for &QualName {
    fn ns(&self) -> &xml5ever::Namespace {
        &self.ns
    }

    fn local_name(&self) -> &xml5ever::LocalName {
        &self.local
    }
}

impl<'arena> Sink<'arena> {
    fn new_node(&self, data: NodeData) -> &'arena mut Node<'arena> {
        self.allocator.alloc(data)
    }
}

impl<'arena> TreeSink for Sink<'arena> {
    type Handle = Ref<'arena>;
    type Output = Ref<'arena>;
    type ElemName<'a>
        = &'a QualName
    where
        Self: 'a;

    fn finish(self) -> Self::Output {
        self.document
    }

    fn parse_error(&self, _msg: std::borrow::Cow<'static, str>) {}

    fn get_document(&self) -> Self::Handle {
        self.document
    }

    fn set_quirks_mode(&self, mode: xml5ever::interface::QuirksMode) {
        self.mode.set(mode);
    }

    fn set_current_line(&self, line: u64) {
        self.line.set(line);
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x == y
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        match target.node_data {
            NodeData::Element { ref name, .. } => name,
            _ => panic!("not an element!"),
        }
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        target
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Self::Handle) -> bool {
        let NodeData::Element { ref name, .. } = handle.node_data else {
            panic!("not an element!");
        };
        name.prefix.is_none() && matches!(name.local.as_ref(), "mi" | "mo" | "mn" | "ms" | "mtext")
    }

    fn create_element(
        &self,
        name: xml5ever::QualName,
        attrs: Vec<xml5ever::Attribute>,
        _flags: xml5ever::interface::ElementFlags,
    ) -> Self::Handle {
        self.new_node(NodeData::Element {
            name: QualName {
                prefix: name.prefix,
                ns: name.ns,
                local: name.local,
            },
            attrs: RefCell::new(
                attrs
                    .into_iter()
                    .map(|attr| Attribute {
                        name: QualName {
                            prefix: attr.name.prefix,
                            ns: attr.name.ns,
                            local: attr.name.local,
                        },
                        value: attr.value,
                    })
                    .collect(),
            ),
            #[cfg(feature = "selectors")]
            selector_flags: Cell::new(None),
        })
    }

    fn create_comment(&self, text: tendril::StrTendril) -> Self::Handle {
        self.new_node(NodeData::Comment(RefCell::new(Some(text))))
    }

    fn create_pi(&self, target: tendril::StrTendril, data: tendril::StrTendril) -> Self::Handle {
        self.new_node(NodeData::PI {
            target,
            value: RefCell::new(Some(data)),
        })
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendNode(node) => {
                parent.append_child(node);
                self.add_xmlns(&node);
                debug_assert!(parent
                    .last_child
                    .get()
                    .is_some_and(|child| std::ptr::eq(child, node)));
            }
            NodeOrText::AppendText(text) => {
                if text.is_empty() {
                    return;
                }
                if let Some(Node {
                    node_data: NodeData::Text(prev_text),
                    ..
                }) = parent.last_child()
                {
                    if let Some(prev_text) = &mut *prev_text.borrow_mut() {
                        prev_text.push_tendril(&text);
                        return;
                    }
                }
                let node = self.new_node(NodeData::Text(RefCell::new(Some(text))));
                parent.append_child(node);
                debug_assert!(parent
                    .last_child
                    .get()
                    .is_some_and(|child| std::ptr::eq(child, node)));
            }
        }
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let mut parent = sibling
            .parent_node()
            .expect("parsed sibling should have parent");
        match new_node {
            NodeOrText::AppendNode(node) => {
                parent.insert_before(node, sibling);
                debug_assert!(sibling
                    .previous_sibling
                    .get()
                    .is_some_and(|child| std::ptr::eq(child, node)));
                debug_assert!(node
                    .next_sibling
                    .get()
                    .is_some_and(|child| std::ptr::eq(child, *sibling)));
            }
            NodeOrText::AppendText(mut text) => {
                text = text.trim().into();
                if text.is_empty() {
                    return;
                }
                let node = self.new_node(NodeData::Text(RefCell::new(Some(text))));
                parent.insert_before(node, sibling);
                debug_assert!(sibling
                    .previous_sibling
                    .get()
                    .is_some_and(|child| std::ptr::eq(child, node)));
                debug_assert!(node
                    .next_sibling
                    .get()
                    .is_some_and(|child| std::ptr::eq(child, *sibling)));
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        if element.parent.get().is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: tendril::StrTendril,
        _public_id: tendril::StrTendril,
        _system_id: tendril::StrTendril,
    ) {
        // doctype not needed in svg documents
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, new_attrs: Vec<xml5ever::Attribute>) {
        let NodeData::Element { ref attrs, .. } = target.node_data else {
            panic!("not an element!");
        };
        let mut attrs = attrs.borrow_mut();

        let existing_names = attrs
            .iter()
            .map(|attr| attr.name.clone())
            .collect::<HashSet<_>>();
        attrs.extend(
            new_attrs
                .into_iter()
                .map(|attr| Attribute {
                    name: QualName {
                        prefix: attr.name.prefix,
                        ns: attr.name.ns,
                        local: attr.name.local,
                    },
                    value: attr.value,
                })
                .filter(|attr| existing_names.contains(&attr.name)),
        );
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        target.remove();
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let mut current = node.first_child.take();
        let old_last_child = new_parent.last_child.take();
        if let Some(current) = current {
            if let Some(old_last_child) = old_last_child {
                old_last_child.next_sibling.set(Some(current));
                current.previous_sibling.set(Some(old_last_child));
            } else {
                debug_assert!(new_parent.first_child.get().is_none());
                new_parent.first_child.set(Some(current));
            }
        } else {
            return;
        }

        while let Some(child) = current {
            child.parent.set(Some(new_parent));
            current = child.next_sibling.get();
        }
        new_parent.last_child.set(current);
    }
}

#[test]
fn parse_markup5ever() {}
