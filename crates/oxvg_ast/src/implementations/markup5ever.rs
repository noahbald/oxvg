//! Parsing methods using xml5ever
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

use xml5ever::{
    driver::{parse_document, XmlParseOpts},
    interface::{NodeOrText, QuirksMode, TreeSink},
    tendril::TendrilSink,
    tree_builder::ElemName,
};

use crate::node::Node as _;

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

struct Sink<'arena> {
    arena: Arena<'arena>,
    document: Ref<'arena>,
    mode: Cell<QuirksMode>,
    line: Cell<u64>,
}

impl<'arena> Sink<'arena> {
    fn new(arena: Arena<'arena>) -> Self {
        Self {
            arena,
            document: arena.alloc(Node::new(NodeData::Document)),
            mode: Cell::new(QuirksMode::NoQuirks),
            line: Cell::new(1),
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
        self.arena.alloc(Node::new(data))
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
        x.ptr_eq(y)
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

    fn append(&self, parent: &Self::Handle, child: xml5ever::interface::NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendNode(node) => parent.append_child(node),
            NodeOrText::AppendText(text) => {
                parent.append_child(self.new_node(NodeData::Text(RefCell::new(Some(text)))));
            }
        }
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let new_node = match new_node {
            NodeOrText::AppendNode(node) => node,
            NodeOrText::AppendText(text) => self.new_node(NodeData::Text(RefCell::new(Some(text)))),
        };
        let before_sibling = sibling.previous_sibling.replace(Some(new_node));
        let parent = sibling.parent.get();

        new_node.next_sibling.set(Some(sibling));
        new_node.parent.set(parent);
        if let Some(before_sibling) = before_sibling {
            before_sibling.next_sibling.set(Some(new_node));
        } else if let Some(parent) = parent {
            parent.first_child.set(Some(new_node));
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
