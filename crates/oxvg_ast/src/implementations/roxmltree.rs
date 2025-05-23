//! Parsing methods using roxmltree
//!
//! # Quirks
//!
//! Roxmltree has some notable quirks
//!
//! - Default PI is skipped
//! - Duplicate namespace uris are merged
use std::{cell::RefCell, collections::HashMap, fmt::Display};

use tendril::StrTendril;
use xml5ever::{local_name, namespace_prefix, namespace_url, ns, Prefix};

use crate::attribute::Attr;

use super::shared::{Arena, Attribute, Node, NodeData, QualName, Ref};

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
struct NamespaceMap {
    prefix_to_uri: HashMap<Option<Prefix>, Option<StrTendril>>,
    uri_to_prefix: HashMap<Option<StrTendril>, Option<Prefix>>,
}

#[allow(clippy::ref_option)]
impl NamespaceMap {
    fn new() -> Self {
        Self::default()
    }

    fn insert(
        &mut self,
        prefix: Option<Prefix>,
        uri: Option<StrTendril>,
    ) -> Option<(Option<Prefix>, Option<StrTendril>)> {
        let p = self.uri_to_prefix.insert(uri.clone(), prefix.clone());
        let u = self.prefix_to_uri.insert(prefix, uri);
        Some((p?, u?))
    }

    fn get_by_uri(&self, uri: &Option<StrTendril>) -> Option<&Option<Prefix>> {
        self.uri_to_prefix.get(uri)
    }

    fn get_by_prefix(&self, prefix: &Option<Prefix>) -> Option<&Option<StrTendril>> {
        self.prefix_to_uri.get(prefix)
    }
}

impl<'arena> Allocator<'arena> {
    fn alloc(&mut self, node_data: NodeData) -> &'arena mut Node<'arena> {
        let id = self.current_node_id;
        self.current_node_id += 1;
        self.arena.alloc(Node::new(node_data, id))
    }
}

/// parse an xml file using roxmltree as the parser.
///
/// # Errors
///
/// If the file cannot be read or parsed
pub fn parse_file<'arena>(
    source: &std::path::Path,
    arena: Arena<'arena>,
) -> Result<Ref<'arena>, ParseError> {
    parse(
        &std::fs::read_to_string(source).map_err(ParseError::IO)?,
        arena,
    )
}

/// parse an xml document using roxmltree as the parser.
///
/// # Errors
///
/// If the string cannot be parsed
pub fn parse<'arena>(source: &str, arena: Arena<'arena>) -> Result<Ref<'arena>, ParseError> {
    let xml = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            // WARN: DOS risk
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .map_err(ParseError::ROXML)?;
    parse_roxmltree(&xml, arena)
}

/// parse an xml document already in roxmltree representation
///
/// # Errors
///
/// If the depth of the tree is too deep
pub fn parse_roxmltree<'arena>(
    xml: &roxmltree::Document,
    arena: Arena<'arena>,
) -> Result<Ref<'arena>, ParseError> {
    let mut id_map = HashMap::new();
    for node in xml.descendants() {
        if let Some(id) = node.attribute("id") {
            if !id_map.contains_key(id) {
                id_map.insert(id, node);
            }
        }
    }

    let mut namespace_map = NamespaceMap::new();
    namespace_map.insert(
        Some(namespace_prefix!("xml")),
        Some("http://www.w3.org/XML/1998/namespace".into()),
    );

    let mut allocator = Allocator {
        arena,
        current_node_id: arena.len(),
    };
    let document = allocator.alloc(NodeData::Document);

    let result =
        parse_xml_node_children(document, &mut allocator, xml.root(), 0, &mut namespace_map);
    result
}

fn create_root<'arena>(arena: &mut Allocator<'arena>) -> &'arena mut Node<'arena> {
    arena.alloc(NodeData::Root)
}

fn parse_xml_node_children<'arena>(
    node: Ref<'arena>,
    allocator: &mut Allocator<'arena>,
    parent: roxmltree::Node<'_, '_>,
    depth: u32,
    namespace_map: &mut NamespaceMap,
) -> Result<Ref<'arena>, ParseError> {
    for xml_child in parent.children() {
        let child = parse_xml_node(allocator, xml_child, depth, namespace_map)?;

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

    Ok(node)
}

fn parse_xml_node<'arena>(
    allocator: &mut Allocator<'arena>,
    node: roxmltree::Node<'_, '_>,
    depth: u32,
    namespace_map: &mut NamespaceMap,
) -> Result<&'arena Node<'arena>, ParseError> {
    if depth > 1024 {
        return Err(ParseError::NodesLimitReached);
    }

    let mut popped_ns: Vec<(Option<Prefix>, Option<StrTendril>)> = vec![];
    let child = &*match node.node_type() {
        roxmltree::NodeType::Root => create_root(allocator),
        roxmltree::NodeType::PI => parse_pi(allocator, node.pi().unwrap()),
        roxmltree::NodeType::Element => {
            parse_element(allocator, node, namespace_map, &mut popped_ns)
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

fn parse_element<'arena>(
    arena: &mut Allocator<'arena>,
    xml_node: roxmltree::Node<'_, '_>,
    namespace_map: &mut NamespaceMap,
    popped_ns: &mut Vec<(Option<Prefix>, Option<StrTendril>)>,
) -> &'arena mut Node<'arena> {
    let xmlns: Vec<_> = xml_node
        .namespaces()
        .filter_map(|ns| find_new_xmlns(ns, namespace_map, popped_ns))
        .collect();
    let attrs =
        xmlns
            .into_iter()
            .chain(xml_node.attributes().map(|attr| {
                Attribute::new(parse_attr_name(attr, namespace_map), attr.value().into())
            }))
            .collect();

    arena.alloc(NodeData::Element {
        name: parse_expanded_name(xml_node.tag_name(), namespace_map),
        attrs: RefCell::new(attrs),
        #[cfg(feature = "selectors")]
        selector_flags: std::cell::Cell::new(None),
    })
}

fn parse_pi<'arena>(
    allocator: &mut Allocator<'arena>,
    pi: roxmltree::PI,
) -> &'arena mut Node<'arena> {
    allocator.alloc(NodeData::PI {
        target: pi.target.into(),
        value: RefCell::new(pi.value.map(Into::into)),
    })
}

fn parse_comment<'arena>(
    allocator: &mut Allocator<'arena>,
    comment: roxmltree::Node,
) -> &'arena mut Node<'arena> {
    allocator.alloc(NodeData::Comment(RefCell::new(
        comment.text().map(Into::into),
    )))
}

fn parse_text<'arena>(
    arena: &mut Allocator<'arena>,
    text: roxmltree::Node,
) -> &'arena mut Node<'arena> {
    arena.alloc(NodeData::Text(RefCell::new(text.text().map(Into::into))))
}

fn parse_attr_name(attr: roxmltree::Attribute, namespace_map: &NamespaceMap) -> QualName {
    let ns = attr.namespace();
    QualName {
        prefix: namespace_map
            .get_by_uri(&ns.map(Into::into))
            .cloned()
            .flatten(),
        ns: ns.map_or_else(
            || {
                namespace_map
                    .get_by_prefix(&None)
                    .cloned()
                    .flatten()
                    .unwrap_or_default()
                    .as_ref()
                    .into()
            },
            Into::into,
        ),
        local: attr.name().into(),
    }
}

fn parse_expanded_name(
    name: roxmltree::ExpandedName,
    namespace_map: &mut NamespaceMap,
) -> QualName {
    let ns = name.namespace();
    QualName {
        prefix: namespace_map
            .get_by_uri(&ns.map(Into::into))
            .cloned()
            .flatten(),
        ns: ns.map_or_else(
            || {
                namespace_map
                    .get_by_prefix(&None)
                    .cloned()
                    .flatten()
                    .unwrap_or_default()
                    .as_ref()
                    .into()
            },
            Into::into,
        ),
        local: name.name().into(),
    }
}

#[allow(clippy::option_option)]
fn find_new_xmlns(
    ns: &roxmltree::Namespace,
    namespace_map: &mut NamespaceMap,
    popped_ns: &mut Vec<(Option<Prefix>, Option<StrTendril>)>,
) -> Option<Attribute> {
    if (ns.name().is_some() || !ns.uri().is_empty()) && !find_xml_uri(ns, namespace_map) {
        let prefix = ns.name().map(Into::into);
        let uri: StrTendril = ns.uri().into();
        if prefix.is_none()
            || (prefix.is_some() && namespace_map.get_by_prefix(&None) != Some(&Some(uri.clone())))
        {
            let popped = namespace_map.insert(prefix.clone(), Some(uri.clone()));
            if let Some(popped) = popped {
                popped_ns.push(popped);
            }
        }
        Some(Attribute {
            name: QualName {
                local: if let Some(prefix) = prefix.as_ref() {
                    prefix.as_ref().into()
                } else {
                    local_name!("xmlns")
                },
                prefix: prefix.as_ref().map(|_| namespace_prefix!("xmlns")),
                ns: ns!(xml),
            },
            value: uri,
        })
    } else {
        None
    }
}

fn find_xml_uri(ns: &roxmltree::Namespace, namespace_map: &mut NamespaceMap) -> bool {
    if let Some(Some(el)) = namespace_map.get_by_prefix(&ns.name().map(Into::into)) {
        el.as_ref() == ns.uri()
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
