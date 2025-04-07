//! Parsing methods using roxmltree
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use string_cache::Atom;

use crate::attribute::Attr;

use super::shared::{Arena, Attribute, Node, NodeData, QualName, Ref};

/// The errors which may occur while parsing a document with roxmltree.
pub enum ParseError {
    /// The document parsed had a depth than 1024 elements
    NodesLimitReached,
    /// The document couldn't be parsed by roxmltree
    ROXML(roxmltree::Error),
    /// The document couldn't be parsed due to an IO issue
    IO(std::io::Error),
}

/// parse an xml file using roxmltree as the parser.
pub fn parse_file<'arena>(
    source: &std::path::Path,
    arena: &Arena<'arena>,
) -> Result<Ref<'arena>, ParseError> {
    parse(
        &std::fs::read_to_string(source).map_err(ParseError::IO)?,
        arena,
    )
}

/// parse an xml document using roxmltree as the parser.
pub fn parse<'arena>(source: &str, arena: &Arena<'arena>) -> Result<Ref<'arena>, ParseError> {
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
pub fn parse_roxmltree<'arena>(
    xml: &roxmltree::Document,
    arena: &Arena<'arena>,
) -> Result<Ref<'arena>, ParseError> {
    let mut id_map = HashMap::new();
    for node in xml.descendants() {
        if let Some(id) = node.attribute("id") {
            if !id_map.contains_key(id) {
                id_map.insert(id, node);
            }
        }
    }

    let mut prefix_map = HashMap::new();
    for ns in xml.root().namespaces() {
        if let Some(prefix) = ns.name() {
            if !prefix_map.contains_key(ns.uri()) {
                prefix_map.insert(ns.uri(), prefix);
            }
        }
    }

    let document = arena.alloc(Node {
        parent: Cell::new(None),
        next_sibling: Cell::new(None),
        previous_sibling: Cell::new(None),
        first_child: Cell::new(None),
        last_child: Cell::new(None),
        node_data: NodeData::Document,
    });

    parse_xml_node_children(document, arena, xml.root(), 0, &prefix_map)
}

fn create_root<'arena>(arena: Arena<'arena>) -> &'arena mut Node<'arena> {
    arena.alloc(Node::new(NodeData::Root))
}

fn parse_xml_node_children<'arena, 'input>(
    node: Ref<'arena>,
    arena: &Arena<'arena>,
    parent: roxmltree::Node<'_, 'input>,
    depth: u32,
    prefix_map: &HashMap<&str, &str>,
) -> Result<Ref<'arena>, ParseError> {
    for child in parent.children() {
        let child = parse_xml_node(arena, child, depth, prefix_map)?;
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

fn parse_xml_node<'arena, 'input>(
    arena: &Arena<'arena>,
    node: roxmltree::Node<'_, 'input>,
    depth: u32,
    prefix_map: &HashMap<&str, &str>,
) -> Result<&'arena mut Node<'arena>, ParseError> {
    if depth > 1024 {
        return Err(ParseError::NodesLimitReached);
    }

    Ok(match node.node_type() {
        roxmltree::NodeType::Root => create_root(arena),
        roxmltree::NodeType::PI => parse_pi(arena, node.pi().unwrap()),
        roxmltree::NodeType::Element => parse_element(arena, node, prefix_map),
        roxmltree::NodeType::Comment => parse_comment(arena, node),
        roxmltree::NodeType::Text => parse_text(arena, node),
    })
}

fn parse_element<'arena, 'input>(
    arena: &Arena<'arena>,
    xml_node: roxmltree::Node<'_, 'input>,
    prefix_map: &HashMap<&str, &str>,
) -> &'arena mut Node<'arena> {
    let attrs = xml_node
        .attributes()
        .map(|attr| Attribute::new(parse_attr_name(attr, prefix_map), attr.value().into()))
        .collect();

    arena.alloc(Node::new(NodeData::Element {
        name: parse_expanded_name(xml_node.tag_name(), prefix_map),
        attrs: RefCell::new(attrs),
        #[cfg(feature = "selectors")]
        selector_flags: Cell::new(None),
    }))
}

fn parse_pi<'arena>(arena: &Arena<'arena>, pi: roxmltree::PI) -> &'arena mut Node<'arena> {
    arena.alloc(Node::new(NodeData::PI {
        target: pi.target.into(),
        value: RefCell::new(pi.value.map(Into::into)),
    }))
}

fn parse_comment<'arena>(
    arena: &Arena<'arena>,
    comment: roxmltree::Node,
) -> &'arena mut Node<'arena> {
    arena.alloc(Node::new(NodeData::Comment(RefCell::new(
        comment.text().map(Into::into),
    ))))
}

fn parse_text<'arena>(arena: &Arena<'arena>, text: roxmltree::Node) -> &'arena mut Node<'arena> {
    arena.alloc(Node::new(NodeData::Text(RefCell::new(
        text.text().map(Into::into),
    ))))
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
    if let Some(ns) = name.namespace() {
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
