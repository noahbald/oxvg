//! Parsing methods using roxmltree
//!
//! # Quirks
//!
//! Roxmltree has some notable quirks
//!
//! - Default PI is skipped
//! - Duplicate namespace uris are merged
use std::{cell::RefCell, collections::HashMap, fmt::Display};

use lightningcss::stylesheet::{ParserOptions, StyleSheet};

use crate::{
    attribute::data::Attr,
    node::{Node, NodeData, Ref},
};

use crate::{
    arena::Arena,
    attribute::data::AttrId,
    element::data::ElementId,
    name::{Prefix, QualName},
};

#[derive(Debug)]
/// The errors which may occur while parsing a document with roxmltree.
pub enum ParseError {
    /// The document parsed had a depth than 1024 elements
    NodesLimitReached,
    /// The document couldn't be parsed by roxmltree
    ROXML(roxmltree::Error),
    /// The document couldn't be parsed due to an IO issue
    IO(std::io::Error),
}

struct Allocator<'arena> {
    arena: Arena<'arena>,
    current_node_id: usize,
}

#[derive(Debug, Default)]
struct NamespaceMap<'input> {
    prefix_to_uri: HashMap<Option<&'input str>, Option<&'input str>>,
    uri_to_prefix: HashMap<Option<&'input str>, Option<&'input str>>,
}

#[allow(clippy::ref_option)]
impl<'input> NamespaceMap<'input> {
    fn new() -> Self {
        Self::default()
    }

    fn insert(
        &mut self,
        prefix: Option<&'input str>,
        uri: Option<&'input str>,
    ) -> Option<(Option<&'input str>, Option<&'input str>)> {
        let p = self.uri_to_prefix.insert(uri.clone(), prefix.clone());
        let u = self.prefix_to_uri.insert(prefix, uri);
        Some((p?, u?))
    }

    fn get_by_uri(&self, uri: &Option<&'input str>) -> Option<&Option<&'input str>> {
        self.uri_to_prefix.get(uri)
    }

    fn get_by_prefix(&self, prefix: &Option<&'input str>) -> Option<&Option<&'input str>> {
        self.prefix_to_uri.get(prefix)
    }
}

impl<'arena> Allocator<'arena> {
    fn alloc(&mut self, node_data: NodeData<'arena>) -> &'arena mut Node<'arena> {
        let id = self.current_node_id;
        self.current_node_id += 1;
        self.arena.alloc(Node::new(node_data, id))
    }
}

/// parse an xml document already in roxmltree representation
///
/// # Errors
///
/// If the depth of the tree is too deep
pub fn parse<'a, 'input: 'a>(
    xml: &'a roxmltree::Document<'input>,
    arena: Arena<'a>,
) -> Result<Ref<'a>, ParseError> {
    let mut namespace_map = NamespaceMap::new();
    namespace_map.insert(Some("xml"), Some("http://www.w3.org/XML/1998/namespace"));

    let mut allocator = Allocator {
        arena,
        current_node_id: arena.len(),
    };
    let document = allocator.alloc(NodeData::Document);

    let result =
        parse_xml_node_children(document, &mut allocator, xml.root(), 0, &mut namespace_map);
    result
}

fn create_root<'input>(arena: &mut Allocator<'input>) -> &'input mut Node<'input> {
    arena.alloc(NodeData::Root)
}

fn parse_xml_node_children<'a, 'input: 'a>(
    node: Ref<'a>,
    allocator: &mut Allocator<'a>,
    parent: roxmltree::Node<'a, 'input>,
    depth: u32,
    namespace_map: &mut NamespaceMap<'a>,
) -> Result<Ref<'a>, ParseError> {
    for xml_child in parent.children() {
        let child = parse_xml_node(allocator, xml_child, depth, namespace_map)?;

        attach_child(node, child);
    }

    Ok(node)
}

fn attach_child<'a>(node: Ref<'a>, child: Ref<'a>) {
    // parent
    child.parent.set(Some(node));

    // parent children
    let last_child = node.last_child.replace(Some(child));
    if node.first_child.get().is_none() {
        node.first_child.set(Some(child));
    }

    // siblings
    child.previous_sibling.set(last_child);
    if let Some(last_child) = last_child {
        last_child.next_sibling.set(Some(child));
    }
}

fn parse_xml_node<'a, 'input: 'a>(
    allocator: &mut Allocator<'a>,
    node: roxmltree::Node<'a, 'input>,
    depth: u32,
    namespace_map: &mut NamespaceMap<'a>,
) -> Result<&'a Node<'a>, ParseError> {
    if depth > 1024 {
        return Err(ParseError::NodesLimitReached);
    }

    let mut popped_ns: Vec<(Option<&'a str>, Option<&'a str>)> = vec![];
    let child = &*match node.node_type() {
        roxmltree::NodeType::Root => create_root(allocator),
        roxmltree::NodeType::PI => parse_pi(allocator, node.pi().unwrap()),
        roxmltree::NodeType::Element => {
            let (child, style) = parse_element(allocator, node, namespace_map, &mut popped_ns);
            if style.is_some() {
                return Ok(child);
            }
            child
        }
        roxmltree::NodeType::Comment => parse_comment(allocator, node),
        roxmltree::NodeType::Text => parse_text(allocator, node),
    };
    parse_xml_node_children(child, allocator, node, depth + 1, namespace_map)?;
    for (prefix, value) in popped_ns {
        namespace_map.insert(prefix, value);
    }
    Ok(child)
}

fn parse_element<'a, 'input: 'a>(
    arena: &mut Allocator<'a>,
    xml_node: roxmltree::Node<'a, 'input>,
    namespace_map: &mut NamespaceMap<'a>,
    popped_ns: &mut Vec<(Option<&'a str>, Option<&'a str>)>,
) -> (Ref<'a>, Option<Ref<'a>>) {
    let xmlns: Vec<_> = xml_node
        .namespaces()
        .filter_map(|ns| find_new_xmlns(ns, namespace_map, popped_ns))
        .collect();
    let attrs = xmlns
        .into_iter()
        .chain(
            xml_node
                .attributes()
                .map(|attr| parse_attr(attr, namespace_map, attr.value())),
        )
        .collect();

    let name = parse_expanded_name(xml_node.tag_name(), namespace_map);
    let is_style_element = name == ElementId::Style;
    let element = NodeData::Element {
        name,
        attrs: RefCell::new(attrs),
        #[cfg(feature = "selectors")]
        selector_flags: std::cell::Cell::new(None),
    };
    let node = arena.alloc(element);
    if is_style_element {
        (node, parse_style(arena, &xml_node, node))
    } else {
        (node, None)
    }
}

fn parse_style<'a, 'input: 'a>(
    arena: &mut Allocator<'a>,
    xml_parent: &roxmltree::Node<'a, '_>,
    parent: Ref<'a>,
) -> Option<Ref<'a>> {
    let Some(style) = xml_parent.text() else {
        return None;
    };
    let style = StyleSheet::parse(style, ParserOptions::default());
    let Ok(style) = style else { return None };
    let style = NodeData::Style(style.rules);
    let node = arena.alloc(style);
    attach_child(parent, node);
    Some(node)
}

fn parse_pi<'input>(
    allocator: &mut Allocator<'input>,
    pi: roxmltree::PI<'input>,
) -> &'input mut Node<'input> {
    allocator.alloc(NodeData::PI {
        target: pi.target.into(),
        value: RefCell::new(pi.value.map(Into::into)),
    })
}

fn parse_comment<'a, 'input: 'a>(
    allocator: &mut Allocator<'a>,
    comment: roxmltree::Node<'a, 'input>,
) -> &'a mut Node<'a> {
    allocator.alloc(NodeData::Comment(RefCell::new(
        comment.text().map(Into::into),
    )))
}

fn parse_text<'a, 'input: 'a>(
    arena: &mut Allocator<'a>,
    text: roxmltree::Node<'a, 'input>,
) -> &'a mut Node<'a> {
    arena.alloc(NodeData::Text(RefCell::new(text.text().map(Into::into))))
}

fn parse_attr<'a, 'input: 'a>(
    attr: roxmltree::Attribute<'a, 'input>,
    namespace_map: &NamespaceMap<'input>,
    value: &'input str,
) -> Attr<'a> {
    let ns = attr.namespace();
    let prefix = namespace_map.get_by_uri(&ns).cloned().flatten();
    let ns = ns.map_or_else(
        || {
            namespace_map
                .get_by_prefix(&None)
                .cloned()
                .flatten()
                .unwrap_or_default()
                .into()
        },
        Into::into,
    );
    let prefix = Prefix::new(ns, prefix.map(Into::into));
    let name = AttrId::new(prefix, attr.name().into());
    Attr::new(name, value.into())
}

fn parse_expanded_name<'a, 'input: 'a>(
    name: roxmltree::ExpandedName<'a, 'input>,
    namespace_map: &mut NamespaceMap<'input>,
) -> ElementId<'a> {
    let ns = name.namespace();
    let prefix = namespace_map
        .get_by_uri(&ns.map(Into::into))
        .cloned()
        .flatten();
    let ns = ns.map_or_else(
        || {
            namespace_map
                .get_by_prefix(&None)
                .cloned()
                .flatten()
                .unwrap_or_default()
                .into()
        },
        Into::into,
    );
    let prefix = Prefix::new(ns, prefix.map(Into::into));
    ElementId::new(prefix, name.name().into())
}

#[allow(clippy::option_option)]
/// When a new value for `ns` is found, add it to `namespace_map`
/// and return an `xmlns` attribute to add to the source element.
fn find_new_xmlns<'a, 'input: 'a>(
    ns: &'a roxmltree::Namespace<'input>,
    namespace_map: &mut NamespaceMap<'a>,
    popped_ns: &mut Vec<(Option<&'a str>, Option<&'a str>)>,
) -> Option<Attr<'a>> {
    if find_xml_uri(ns, namespace_map) {
        return None;
    }
    let uri = ns.uri();
    if let Some(prefix) = ns.name() {
        if namespace_map.get_by_prefix(&None) != Some(&Some(uri)) {
            if let Some(popped) = namespace_map.insert(Some(prefix), Some(uri)) {
                popped_ns.push(popped);
            }
        }
        // return `xmlns:ns="uri"`
        Some(Attr::Unparsed {
            attr_id: AttrId::Unknown(QualName {
                prefix: Prefix::XMLNS,
                local: prefix.into(),
            }),
            value: uri.into(),
        })
    } else if !ns.uri().is_empty() {
        if let Some(popped) = namespace_map.insert(None, Some(uri)) {
            popped_ns.push(popped);
        }
        // return `xmlns="uri"`
        Some(Attr::XMLNS(uri.into()))
    } else {
        // no new xmlns
        None
    }
}

fn find_xml_uri(ns: &roxmltree::Namespace, namespace_map: &mut NamespaceMap) -> bool {
    if let Some(Some(el)) = namespace_map.get_by_prefix(&ns.name()) {
        *el == ns.uri()
    } else {
        false
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodesLimitReached => f.write_str("The depth of the document parsed was too deep"),
            Self::ROXML(err) => err.fmt(f),
            Self::IO(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ParseError {}
