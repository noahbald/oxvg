//! Parsing methods using DomParser from the JavaScript environment
use std::{cell::RefCell, fmt::Display};

use lightningcss::stylesheet::{ParserFlags, ParserOptions, StyleSheet};
use oxvg_collections::{
    attribute::Attr,
    element::ElementId,
    name::{Prefix, NS},
};
use web_sys::{
    wasm_bindgen::{JsCast, JsValue},
    DomParser, SupportedType,
};

use crate::{
    arena::{Allocator, Arena, Values},
    node::{NodeData, Ref},
};

pub use web_sys::Document;

#[derive(Debug)]
/// The errors which may occur while parsing a document with roxmltree.
pub enum ParseError {
    /// [`DomParser`] was unable to be constructed
    DomParserError(JsValue),
    /// [`DomParser`] was unable to parse the document
    ParseError(JsValue),
    /// OXVG was unable to parse a part of the document, usually because it would cause
    /// inconsistency with non-web parsers.
    Unsupported(String),
    /// The document parsed had a depth than 1024 elements
    NodesLimitReached,
}

/// parse an xml document already in roxmltree representation with an allocator
///
/// # Errors
///
/// If the depth of the tree is too deep
pub fn parse_tree_with_allocator<'input, 'arena, T, F>(
    source: &Document,
    arena: &'arena mut Arena<'input, 'arena>,
    values: &'input Values,
    mut f: F,
) -> Result<T, ParseError>
where
    F: FnMut(Ref<'input, 'arena>, Allocator<'input, 'arena>) -> T,
{
    let mut allocator = Allocator::new(arena, values);

    let document = allocator.alloc(NodeData::Document);
    let document = parse_xml_node_children(document, &mut allocator, source.as_ref(), 0)?;

    Ok(f(document, allocator))
}

/// parse an xml document already in roxmltree representation
///
/// # Errors
///
/// If the depth of the tree is too deep
pub fn parse_tree<
    T,
    F: for<'input, 'arena> FnMut(Ref<'input, 'arena>, Allocator<'input, 'arena>) -> T,
>(
    source: &Document,
    f: F,
) -> Result<T, ParseError> {
    let values = Allocator::new_values();
    let mut arena = Allocator::new_arena();

    parse_tree_with_allocator(source, &mut arena, &values, f)
}

/// parse an xml document using roxmltree
///
/// # Errors
///
/// If the depth of the tree is too deep
pub fn parse<
    T,
    F: for<'input, 'arena> FnMut(Ref<'input, 'arena>, Allocator<'input, 'arena>) -> T,
>(
    source: &str,
    f: F,
) -> Result<T, ParseError> {
    let parser = DomParser::new().map_err(ParseError::DomParserError)?;
    let document = parser
        .parse_from_string(source, SupportedType::ImageSvgXml)
        .map_err(ParseError::ParseError)?;
    parse_tree(&document, f)
}

fn parse_xml_node_children<'input, 'arena>(
    node: Ref<'input, 'arena>,
    allocator: &mut Allocator<'input, 'arena>,
    parent: &web_sys::Node,
    depth: u32,
) -> Result<Ref<'input, 'arena>, ParseError> {
    let children = parent.child_nodes();
    let mut i = 0;
    while let Some(child) = children.get(i) {
        let child = parse_xml_node(allocator, child, depth)?;

        attach_child(node, child);
        i += 1;
    }

    Ok(node)
}

fn attach_child<'input, 'arena>(node: Ref<'input, 'arena>, child: Ref<'input, 'arena>) {
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

fn parse_xml_node<'input, 'arena>(
    allocator: &mut Allocator<'input, 'arena>,
    node: web_sys::Node,
    depth: u32,
) -> Result<Ref<'input, 'arena>, ParseError> {
    if depth > 1024 {
        return Err(ParseError::NodesLimitReached);
    }

    let child = match node.node_type() {
        1 => {
            let (child, style) = parse_element(allocator, &node.clone().unchecked_into())?;
            if let Some(style) = style {
                attach_child(child, style);
                return Ok(child);
            }
            child
        }
        2 => unreachable!("Attributes handled separately"),
        3 | 4 => parse_text(allocator, &node),
        5 | 6 | 12 => unreachable!("Types no longer in use"),
        7 => parse_pi(allocator, &node.clone().unchecked_into()),
        8 => parse_comment(allocator, &node.clone().unchecked_into()),
        9 | 10 | 11 => create_root(allocator),
        _ => unreachable!("Out of range"),
    };
    parse_xml_node_children(child, allocator, &node, depth + 1)?;
    Ok(child)
}

fn parse_element<'input, 'arena>(
    arena: &mut Allocator<'input, 'arena>,
    web_element: &web_sys::Element,
) -> Result<(Ref<'input, 'arena>, Option<Ref<'input, 'arena>>), ParseError> {
    let name = parse_expanded_name(web_element);

    let named_node_map = web_element.attributes();
    let mut attrs = Vec::with_capacity(named_node_map.length() as usize);
    let mut i = 0;
    while let Some(attr) = named_node_map.get_with_index(i) {
        attrs.push(parse_attr(arena, &name, attr));
        i += 1;
    }
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
        #[cfg(feature = "range")]
        range: None,
        #[cfg(feature = "range")]
        ranges: std::collections::HashMap::default(),
    };
    let node = arena.alloc(element);
    if is_style_element {
        Ok((node, parse_style(arena, &web_element, node)?))
    } else {
        Ok((node, None))
    }
}

fn parse_style<'input, 'arena>(
    arena: &mut Allocator<'input, 'arena>,
    web_parent: &web_sys::Element,
    parent: Ref<'input, 'arena>,
) -> Result<Option<Ref<'input, 'arena>>, ParseError> {
    if web_parent.child_element_count() > 1 {
        // Don't allow inconsistency between DomParser and other parsers
        return Err(ParseError::Unsupported(
            "Multiple style child nodes are not supported".into(),
        ));
    }
    let Some(child) = web_parent.first_child() else {
        return Ok(None);
    };
    let Some(style) = child.text_content() else {
        // Don't allow inconsistency between DomParser and other parsers
        return Err(ParseError::Unsupported(
            "Non-text style children not supported".into(),
        ));
    };
    let options = ParserOptions {
        flags: ParserFlags::all(),
        ..ParserOptions::default()
    };
    let style = arena.alloc_str(&style);
    let node = if let Ok(style) = StyleSheet::parse(style, options) {
        let style = NodeData::Style(RefCell::new(style.rules));
        arena.alloc(style)
    } else {
        parse_text(arena, &child)
    };
    attach_child(parent, node);
    Ok(Some(node))
}

fn parse_pi<'input, 'arena>(
    allocator: &mut Allocator<'input, 'arena>,
    pi: &web_sys::ProcessingInstruction,
) -> Ref<'input, 'arena> {
    let value: web_sys::CharacterData = pi.clone().unchecked_into();
    allocator.alloc(NodeData::PI {
        target: pi.target().into(),
        value: RefCell::new(value.text_content().map(Into::into)),
    })
}

fn parse_comment<'input, 'arena>(
    allocator: &mut Allocator<'input, 'arena>,
    comment: &web_sys::Comment,
) -> Ref<'input, 'arena> {
    allocator.alloc(NodeData::Comment(RefCell::new(Some(comment.data().into()))))
}

fn parse_text<'input, 'arena>(
    arena: &mut Allocator<'input, 'arena>,
    text: &web_sys::Node,
) -> Ref<'input, 'arena> {
    arena.alloc(NodeData::Text(RefCell::new(
        text.text_content().map(Into::into),
    )))
}

fn parse_attr<'input, 'arena>(
    arena: &mut Allocator<'input, 'arena>,
    element: &ElementId<'input>,
    attr: web_sys::Attr,
) -> Attr<'input> {
    let prefix = attr.prefix();
    let ns = attr
        .namespace_uri()
        .map(Into::into)
        .unwrap_or_else(|| NS::SVG.uri().clone());
    let prefix = Prefix::new(ns, prefix.map(Into::into));
    let name = element.parse_attr_id(&prefix, attr.name().into());
    let value = arena.alloc_str(&attr.value());
    Attr::new(name, value)
}

fn parse_expanded_name<'input>(element: &web_sys::Element) -> ElementId<'input> {
    let prefix = element.prefix();
    let ns = element
        .namespace_uri()
        .map(Into::into)
        .unwrap_or_else(|| NS::SVG.uri().clone());
    let prefix = Prefix::new(ns, prefix.map(Into::into));
    ElementId::new(prefix, element.local_name().into())
}

fn create_root<'input, 'arena>(arena: &mut Allocator<'input, 'arena>) -> Ref<'input, 'arena> {
    arena.alloc(NodeData::Root)
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DomParserError(_) => f.write_str("DomParser failed to build"),
            Self::ParseError(_) => f.write_str("DomParser failed to parse"),
            Self::Unsupported(err) => err.fmt(f),
            Self::NodesLimitReached => f.write_str("The depth of the document parsed was too deep"),
        }
    }
}

impl std::error::Error for ParseError {}

#[test]
#[allow(clippy::too_many_lines)]
fn parse_js_sys() {
    use lightningcss::values::{
        color::{CssColor, RGBA},
        length::LengthValue,
        percentage::{DimensionPercentage, Percentage},
    };
    use oxvg_collections::{
        atom::Atom,
        attribute::{core_attrs::Paint, inheritable::Inheritable, presentation::LengthPercentage},
    };

    let source = r#"<svg version="1.1" baseProfile="full" width="300" height="200" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="black" />
  <circle cx="150" cy="100" r="90" fill="blue" />
  <style>rect { fill: blue; }</style>
</svg>"#;
    parse(source, |document, _| {
        let document = document.find_element().unwrap();
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
    })
    .unwrap();
}
