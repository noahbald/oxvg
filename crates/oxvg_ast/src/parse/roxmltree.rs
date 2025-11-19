//! Parsing methods using roxmltree
//!
//! # Quirks
//!
//! Roxmltree has some notable quirks
//!
//! - Default PI is skipped
//! - Duplicate namespace uris are merged
use std::{cell::RefCell, collections::HashMap, fmt::Display};

use lightningcss::{
    rules::CssRuleList,
    stylesheet::{ParserFlags, ParserOptions, StyleSheet},
};
use oxvg_collections::{
    attribute::{Attr, AttrId},
    element::ElementId,
    name::{Prefix, QualName},
};

use crate::{
    arena::Allocator,
    node::{Node, NodeData, Ref},
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
        let p = self.uri_to_prefix.insert(uri, prefix);
        let u = self.prefix_to_uri.insert(prefix, uri);
        Some((p?, u?))
    }

    fn get_by_uri(&self, uri: Option<&'input str>) -> Option<&'input str> {
        self.uri_to_prefix.get(&uri).copied().flatten()
    }

    fn get_by_prefix(&self, prefix: Option<&'input str>) -> Option<&'input str> {
        self.prefix_to_uri.get(&prefix).copied().flatten()
    }
}

/// parse an xml document already in roxmltree representation
///
/// # Errors
///
/// If the depth of the tree is too deep
pub fn parse<'a, 'input: 'a, 'arena>(
    xml: &'a roxmltree::Document<'input>,
    allocator: &mut Allocator<'a, 'arena>,
) -> Result<Ref<'a, 'arena>, ParseError> {
    let mut namespace_map = NamespaceMap::new();
    namespace_map.insert(Some("xml"), Some("http://www.w3.org/XML/1998/namespace"));

    let document = allocator.alloc(NodeData::Document);

    let result = parse_xml_node_children(document, allocator, xml.root(), 0, &mut namespace_map);
    result
}

fn create_root<'input, 'arena>(
    arena: &mut Allocator<'input, 'arena>,
) -> &'arena mut Node<'input, 'arena> {
    arena.alloc(NodeData::Root)
}

fn parse_xml_node_children<'a, 'input: 'a, 'arena>(
    node: Ref<'a, 'arena>,
    allocator: &mut Allocator<'a, 'arena>,
    parent: roxmltree::Node<'a, 'input>,
    depth: u32,
    namespace_map: &mut NamespaceMap<'a>,
) -> Result<Ref<'a, 'arena>, ParseError> {
    for xml_child in parent.children() {
        let child = parse_xml_node(allocator, xml_child, depth, namespace_map)?;

        attach_child(node, child);
    }

    Ok(node)
}

fn attach_child<'a, 'arena>(node: Ref<'a, 'arena>, child: Ref<'a, 'arena>) {
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

fn parse_xml_node<'a, 'input: 'a, 'arena>(
    allocator: &mut Allocator<'a, 'arena>,
    node: roxmltree::Node<'a, 'input>,
    depth: u32,
    namespace_map: &mut NamespaceMap<'a>,
) -> Result<Ref<'a, 'arena>, ParseError> {
    if depth > 1024 {
        return Err(ParseError::NodesLimitReached);
    }

    let mut popped_ns: Vec<(Option<&'a str>, Option<&'a str>)> = vec![];
    let child = match node.node_type() {
        roxmltree::NodeType::Root => create_root(allocator),
        roxmltree::NodeType::PI => parse_pi(allocator, node.pi().unwrap()),
        roxmltree::NodeType::Element => {
            let (child, style) = parse_element(allocator, node, namespace_map, &mut popped_ns);
            if let Some(style) = style {
                attach_child(child, style);
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

fn parse_element<'a, 'input: 'a, 'arena>(
    arena: &mut Allocator<'a, 'arena>,
    xml_node: roxmltree::Node<'a, 'input>,
    namespace_map: &mut NamespaceMap<'a>,
    popped_ns: &mut Vec<(Option<&'a str>, Option<&'a str>)>,
) -> (Ref<'a, 'arena>, Option<Ref<'a, 'arena>>) {
    let name = parse_expanded_name(xml_node.tag_name(), namespace_map);

    let xml_node_namespaces = xml_node.namespaces();
    let xml_node_attributes = xml_node.attributes();
    let mut attrs = Vec::with_capacity(xml_node_attributes.len() + xml_node_namespaces.len());
    attrs.extend(xml_node_namespaces.filter_map(|ns| find_new_xmlns(ns, namespace_map, popped_ns)));
    attrs.extend(
        xml_node_attributes.map(|attr| parse_attr(&name, attr, namespace_map, attr.value())),
    );
    let is_style_element = name == ElementId::Style
        && !attrs.iter().any(|attr| match attr {
            Attr::TypeStyle(r#type) => !r#type.is_empty() && &**r#type != "text/css",
            _ => false,
        });
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

fn parse_style<'a, 'input: 'a, 'arena>(
    arena: &mut Allocator<'a, 'arena>,
    xml_parent: &roxmltree::Node<'a, '_>,
    parent: Ref<'a, 'arena>,
) -> Option<Ref<'a, 'arena>> {
    let styles = xml_parent
        .children()
        // WARN: Skips non-text/non-contentful nodes
        .filter(roxmltree::Node::is_text)
        .filter_map(|child| child.text());
    let mut rules = CssRuleList(vec![]);
    for style in styles {
        let options = ParserOptions {
            flags: ParserFlags::all(),
            ..ParserOptions::default()
        };
        let style = StyleSheet::parse(style, options);
        let Ok(style) = style else { continue };
        rules.0.extend(style.rules.0);
    }
    if rules.0.is_empty() {
        return None;
    }
    let style = NodeData::Style(RefCell::new(rules));
    let node = arena.alloc(style);
    attach_child(parent, node);
    Some(node)
}

fn parse_pi<'input, 'arena>(
    allocator: &mut Allocator<'input, 'arena>,
    pi: roxmltree::PI<'input>,
) -> &'arena mut Node<'input, 'arena> {
    allocator.alloc(NodeData::PI {
        target: pi.target.into(),
        value: RefCell::new(pi.value.map(Into::into)),
    })
}

fn parse_comment<'a, 'input: 'a, 'arena>(
    allocator: &mut Allocator<'a, 'arena>,
    comment: roxmltree::Node<'a, 'input>,
) -> &'arena mut Node<'a, 'arena> {
    allocator.alloc(NodeData::Comment(RefCell::new(
        comment.text().map(Into::into),
    )))
}

fn parse_text<'a, 'input: 'a, 'arena>(
    arena: &mut Allocator<'a, 'arena>,
    text: roxmltree::Node<'a, 'input>,
) -> &'arena mut Node<'a, 'arena> {
    arena.alloc(NodeData::Text(RefCell::new(text.text().map(Into::into))))
}

fn parse_attr<'a, 'input: 'a>(
    element: &ElementId<'input>,
    attr: roxmltree::Attribute<'a, 'input>,
    namespace_map: &NamespaceMap<'input>,
    value: &'input str,
) -> Attr<'a> {
    let ns = attr.namespace();
    let prefix = namespace_map.get_by_uri(ns);
    let ns = ns.map_or_else(
        || namespace_map.get_by_prefix(None).unwrap_or_default().into(),
        Into::into,
    );
    let prefix = Prefix::new(ns, prefix.map(Into::into));
    let name = element.parse_attr_id(&prefix, attr.name().into());
    Attr::new(name, value)
}

fn parse_expanded_name<'a, 'input: 'a>(
    name: roxmltree::ExpandedName<'a, 'input>,
    namespace_map: &NamespaceMap<'input>,
) -> ElementId<'a> {
    let ns = name.namespace();
    let prefix = namespace_map.get_by_uri(ns);
    let ns = ns.map_or_else(
        || namespace_map.get_by_prefix(None).unwrap_or_default().into(),
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
        if namespace_map.get_by_prefix(None) != Some(uri) {
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
    if let Some(el) = namespace_map.get_by_prefix(ns.name()) {
        el == ns.uri()
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

#[test]
#[allow(clippy::too_many_lines)]
fn parse_roxmltree() {
    use lightningcss::values::{
        color::{CssColor, RGBA},
        length::LengthValue,
        percentage::{DimensionPercentage, Percentage},
    };
    use oxvg_collections::{
        atom::Atom,
        attribute::{core::Paint, inheritable::Inheritable, presentation::LengthPercentage},
    };
    let source = r#"<svg version="1.1" baseProfile="full" width="300" height="200" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="black" />
  <circle cx="150" cy="100" r="90" fill="blue" />
  <style>rect { fill: blue; }</style>
</svg>"#;
    let xml = roxmltree::Document::parse(source).unwrap();
    let values = Allocator::new_values();
    let mut arena = Allocator::new_arena();
    let mut allocator = Allocator::new(&mut arena, &values);
    let document = parse(&xml, &mut allocator).unwrap().find_element().unwrap();

    assert_eq!(document.qual_name(), &ElementId::Svg);
    let attributes = document.attributes();
    assert_eq!(attributes.len(), 5);
    assert_eq!(
        &*attributes.item(0).unwrap(),
        &Attr::XMLNS("http://www.w3.org/2000/svg".into())
    );
    assert_eq!(
        &*attributes.item(1).unwrap(),
        &Attr::Version(Atom::Static("1.1"))
    );
    assert_eq!(
        &*attributes.item(2).unwrap(),
        &Attr::BaseProfile("full".into())
    );
    assert_eq!(
        &*attributes.item(3).unwrap(),
        &Attr::WidthSvg(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(300.0)
        )))
    );
    assert_eq!(
        &*attributes.item(4).unwrap(),
        &Attr::HeightSvg(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(200.0)
        )))
    );

    let rect = document.first_element_child().unwrap();
    assert_eq!(rect.qual_name(), &ElementId::Rect);
    let attributes = rect.attributes();
    assert_eq!(attributes.len(), 3);
    assert_eq!(
        &*attributes.item(0).unwrap(),
        &Attr::WidthRect(LengthPercentage(DimensionPercentage::Percentage(
            Percentage(1.0)
        )))
    );
    assert_eq!(
        &*attributes.item(1).unwrap(),
        &Attr::HeightRect(LengthPercentage::Percentage(Percentage(1.0)))
    );
    assert_eq!(
        &*attributes.item(2).unwrap(),
        &Attr::Fill(Inheritable::Defined(Paint::Color(CssColor::RGBA(RGBA {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255
        }))))
    );

    let circle = rect.next_element_sibling().unwrap();
    assert_eq!(circle.qual_name(), &ElementId::Circle);
    let attributes = circle.attributes();
    assert_eq!(attributes.len(), 4);
    assert_eq!(
        &*attributes.item(0).unwrap(),
        &Attr::CXGeometry(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(150.0)
        )))
    );
    assert_eq!(
        &*attributes.item(1).unwrap(),
        &Attr::CYGeometry(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(100.0)
        )))
    );
    assert_eq!(
        &*attributes.item(2).unwrap(),
        &Attr::RGeometry(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(90.0)
        )))
    );
    assert_eq!(
        &*attributes.item(3).unwrap(),
        &Attr::Fill(Inheritable::Defined(Paint::Color(CssColor::RGBA(RGBA {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 255
        }))))
    );
}
