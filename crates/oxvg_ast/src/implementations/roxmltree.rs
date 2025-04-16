//! Parsing methods using roxmltree
//!
//! # Quirks
//!
//! Roxmltree has some notable quirks
//!
//! - Default PI is skipped
//! - Duplicate namespace uris are merged
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Display,
};

use string_cache::Atom;
use tendril::StrTendril;
use xml5ever::{local_name, namespace_prefix, namespace_url, ns, Prefix};

use crate::{attribute::Attr, node::Node as _};

use super::shared::{Arena, Attribute, Element, Node, NodeData, QualName, Ref};

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

type NamespaceMap = HashMap<Option<Prefix>, Option<StrTendril>>;

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

    let mut namespace_map = HashMap::new();
    let mut prefix_map = HashMap::from([
        ("xml", "http://www.w3.org/XML/1998/namespace"),
        ("http://www.w3.org/XML/1998/namespace", "xml"),
    ]);
    for ns in xml.descendants().flat_map(|n| n.namespaces()) {
        if let Some(prefix) = ns.name() {
            if !prefix_map.contains_key(ns.uri()) {
                prefix_map.insert(ns.uri(), prefix);
                prefix_map.insert(prefix, ns.uri());
            }
        }
    }

    let mut allocator = Allocator {
        arena,
        current_node_id: arena.len(),
    };
    let document = allocator.alloc(NodeData::Document);

    let result = parse_xml_node_children(
        document,
        &mut allocator,
        xml.root(),
        0,
        &prefix_map,
        &mut namespace_map,
        None,
    );
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
    prefix_map: &HashMap<&str, &str>,
    namespace_map: &mut NamespaceMap,
    root: Option<&Element<'arena>>,
) -> Result<Ref<'arena>, ParseError> {
    for xml_child in parent.children() {
        let child = parse_xml_node(allocator, xml_child, depth, prefix_map, namespace_map)?;
        let child_element = (&*child).element();
        let root = if root.is_some() {
            root
        } else {
            child_element.as_ref()
        };
        parse_xml_node_children(
            child,
            allocator,
            xml_child,
            depth + 1,
            prefix_map,
            namespace_map,
            root,
        )?;

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
    prefix_map: &HashMap<&str, &str>,
    namespace_map: &mut NamespaceMap,
) -> Result<&'arena mut Node<'arena>, ParseError> {
    if depth > 1024 {
        return Err(ParseError::NodesLimitReached);
    }

    Ok(match node.node_type() {
        roxmltree::NodeType::Root => create_root(allocator),
        roxmltree::NodeType::PI => parse_pi(allocator, node.pi().unwrap()),
        roxmltree::NodeType::Element => parse_element(allocator, node, prefix_map, namespace_map),
        roxmltree::NodeType::Comment => parse_comment(allocator, node),
        roxmltree::NodeType::Text => parse_text(allocator, node),
    })
}

fn parse_element<'arena>(
    arena: &mut Allocator<'arena>,
    xml_node: roxmltree::Node<'_, '_>,
    prefix_map: &HashMap<&str, &str>,
    namespace_map: &mut NamespaceMap,
) -> &'arena mut Node<'arena> {
    let attrs = xml_node
        .namespaces()
        .filter_map(|ns| find_new_xmlns(ns, namespace_map))
        .chain(
            xml_node
                .attributes()
                .map(|attr| Attribute::new(parse_attr_name(attr, prefix_map), attr.value().into())),
        )
        .collect();

    arena.alloc(NodeData::Element {
        name: parse_expanded_name(xml_node.tag_name(), prefix_map),
        attrs: RefCell::new(attrs),
        #[cfg(feature = "selectors")]
        selector_flags: Cell::new(None),
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

fn parse_attr_name(attr: roxmltree::Attribute, prefix_map: &HashMap<&str, &str>) -> QualName {
    if let Some(ns) = attr.namespace() {
        QualName {
            prefix: prefix_map.get(ns).map(|prefix| (*prefix).into()),
            ns: ns.into(),
            local: attr.name().into(),
        }
    } else {
        QualName {
            prefix: None,
            ns: Atom::default(),
            local: attr.name().into(),
        }
    }
}

fn parse_expanded_name(
    name: roxmltree::ExpandedName,
    prefix_map: &HashMap<&str, &str>,
) -> QualName {
    let mut ns = name.namespace();
    if ns.is_some_and(str::is_empty) {
        ns = None;
    }
    if let Some(ns) = ns {
        QualName {
            prefix: prefix_map.get(ns).map(|prefix| (*prefix).into()),
            ns: ns.into(),
            local: name.name().into(),
        }
    } else {
        QualName {
            prefix: None,
            ns: Atom::default(),
            local: name.name().into(),
        }
    }
}

fn find_new_xmlns(
    ns: &roxmltree::Namespace,
    namespace_map: &mut NamespaceMap,
) -> Option<Attribute> {
    if (ns.name().is_some() || !ns.uri().is_empty()) && !find_xml_uri(ns, namespace_map) {
        let prefix = ns.name().map(Into::into);
        let uri: StrTendril = ns.uri().into();
        namespace_map.insert(prefix.clone(), Some(uri.clone()));
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
    if let Some(Some(el)) = namespace_map.get(&ns.name().map(Into::into)) {
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
