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

use oxvg_collections::{
    atom::Atom,
    attribute::{Attr, AttrId},
    element::ElementId,
    name::{Prefix, QualName},
};
use xml5ever::{
    driver::{parse_document, XmlParseOpts},
    interface::{NodeOrText, QuirksMode, TreeSink},
    tendril::TendrilSink,
    tree_builder::NamespaceMap,
};

use crate::{
    arena::Allocator,
    element::Element,
    is_attribute,
    node::{Node, NodeData, Ref},
};

/// parse an xml file using xml5ever as the parser.
///
/// # Errors
///
/// If the file cannot be read or parsed
pub fn parse_file<'input, 'arena>(
    source: &std::path::Path,
    allocator: &mut Allocator<'input, 'arena>,
) -> Result<Ref<'input, 'arena>, std::io::Error> {
    parse_document(Sink::new(allocator), XmlParseOpts::default())
        .from_utf8()
        .from_file(source)
}

/// parse an xml document using xml5ever as the parser.
pub fn parse<'input, 'arena>(
    source: &str,
    allocator: &mut Allocator<'input, 'arena>,
) -> Ref<'input, 'arena> {
    parse_document(Sink::new(allocator), XmlParseOpts::default()).one(source)
}

struct Sink<'a, 'input, 'arena> {
    allocator: &'a mut Allocator<'input, 'arena>,
    document: Ref<'input, 'arena>,
    namespace_map: RefCell<NamespaceMap>,
    mode: Cell<QuirksMode>,
    line: Cell<u64>,
}

#[derive(Debug)]
struct ElemName<'a> {
    ns: &'a xml5ever::Namespace,
    local_name: &'a xml5ever::LocalName,
}
impl xml5ever::tree_builder::ElemName for ElemName<'_> {
    fn ns(&self) -> &xml5ever::Namespace {
        self.ns
    }

    fn local_name(&self) -> &xml5ever::LocalName {
        self.local_name
    }
}

impl<'a, 'input, 'arena> Sink<'a, 'input, 'arena> {
    fn new(allocator: &'a mut Allocator<'input, 'arena>) -> Self {
        Self {
            document: allocator.alloc(NodeData::Document),
            allocator,
            namespace_map: RefCell::new(NamespaceMap::empty()),
            mode: Cell::new(QuirksMode::NoQuirks),
            line: Cell::new(1),
        }
    }

    /// Checks whether a namespace is already in the document
    fn find_xml_uri(&self, prefix: &Prefix<'arena>) -> bool {
        if let Some(Some(el)) = self
            .namespace_map
            .borrow()
            .get(&prefix.value().map(|atom| atom.as_str().into()))
        {
            el == prefix.ns().uri().as_str()
        } else {
            false
        }
    }

    /// If a new namespace is found, add it to namespace map and return attribute to add to element
    fn find_new_xmlns(
        &self,
        prefix: &Prefix<'input>,
        local_name: &Atom<'input>,
    ) -> Option<Attr<'input>> {
        if (!prefix.is_empty() || !prefix.ns().uri().is_empty()) && !self.find_xml_uri(prefix) {
            self.namespace_map.borrow_mut().insert(&xml5ever::QualName {
                prefix: prefix.value().map(|atom| atom.as_str().into()),
                ns: prefix.ns().uri().as_str().into(),
                local: local_name.as_str().into(),
            });
            let uri = prefix.ns().uri();
            let prefix = prefix.value();
            Some(if let Some(local) = prefix {
                Attr::Unparsed {
                    attr_id: AttrId::Unknown(QualName {
                        prefix: Prefix::XMLNS,
                        local,
                    }),
                    value: uri.clone(),
                }
            } else {
                Attr::XMLNS(uri.clone())
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
        if let Some(attr_to_insert) = self.find_new_xmlns(name.prefix(), name.local_name()) {
            if attr_to_insert.prefix().is_empty() {
                attrs.borrow_mut().insert(0, attr_to_insert);
            } else {
                root.attributes().set_named_item(attr_to_insert);
            }
        }

        let attrs_ref = attrs.borrow();
        let root_attrs_to_insert: Vec<_> = attrs_ref
            .iter()
            .filter_map(|attr| self.find_new_xmlns(attr.prefix(), attr.local_name()))
            .collect();
        drop(attrs_ref);

        let mut root_attrs = root.attributes().0.borrow_mut();
        let has_xmlns = root_attrs
            .first()
            .is_some_and(|attr| attr.prefix().is_empty() && is_attribute!(attr, XMLNS));
        for (i, attr) in root_attrs_to_insert.into_iter().enumerate() {
            if attr.local_name().as_str() == "xml" {
                continue;
            }
            root_attrs.insert(if has_xmlns { i + 1 } else { i }, attr);
        }
    }
}

impl<'input, 'arena> Sink<'_, 'input, 'arena> {
    fn new_node(&self, data: NodeData<'input>) -> &'arena mut Node<'input, 'arena> {
        self.allocator.alloc(data)
    }
}

impl<'input, 'arena> TreeSink for Sink<'_, 'input, 'arena> {
    type Handle = Ref<'input, 'arena>;
    type Output = Ref<'input, 'arena>;
    type ElemName<'b>
        = ElemName<'b>
    where
        Self: 'b;

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

    fn elem_name<'b>(&'b self, target: &'b Self::Handle) -> Self::ElemName<'b> {
        match target.node_data {
            NodeData::Element { ref name, .. } => {
                let Atom::NS(ns) = name.prefix().ns().uri() else {
                    panic!("Parser created non-interned NS");
                };
                let Atom::Local(local_name) = name.local_name() else {
                    panic!("Parser created non-interned local-name");
                };
                ElemName { ns, local_name }
            }
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
        name.prefix().is_empty()
            && matches!(
                name.local_name().as_ref(),
                "mi" | "mo" | "mn" | "ms" | "mtext"
            )
    }

    fn create_element(
        &self,
        name: xml5ever::QualName,
        attrs: Vec<xml5ever::Attribute>,
        _flags: xml5ever::interface::ElementFlags,
    ) -> Self::Handle {
        let name = ElementId::new(
            Prefix::new(name.ns.into(), name.prefix.map(Atom::Prefix)),
            name.local.into(),
        );
        self.new_node(NodeData::Element {
            attrs: RefCell::new(
                attrs
                    .into_iter()
                    .map(|attr| {
                        Attr::new(
                            name.parse_attr_id(
                                &Prefix::new(
                                    attr.name.ns.into(),
                                    attr.name.prefix.map(Atom::Prefix),
                                ),
                                attr.name.local.into(),
                            ),
                            self.allocator.alloc_str(attr.value.as_ref()),
                        )
                    })
                    .collect(),
            ),
            name,
            #[cfg(feature = "selectors")]
            selector_flags: Cell::new(None),
        })
    }

    fn create_comment(&self, text: tendril::StrTendril) -> Self::Handle {
        self.new_node(NodeData::Comment(RefCell::new(Some(text.into()))))
    }

    fn create_pi(&self, target: tendril::StrTendril, data: tendril::StrTendril) -> Self::Handle {
        self.new_node(NodeData::PI {
            target: target.into(),
            value: RefCell::new(Some(data.into())),
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
                        prev_text.push_str(&text);
                        return;
                    }
                }
                let node = self.new_node(NodeData::Text(RefCell::new(Some(text.into()))));
                parent.append_child(node);
                debug_assert!(parent
                    .last_child
                    .get()
                    .is_some_and(|child| std::ptr::eq(child, node)));
            }
        }
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let parent = sibling
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
                text.pop_front_char_run(char::is_whitespace);
                text.pop_back((text.len() - text.trim_end().len()).try_into().unwrap());
                if text.is_empty() {
                    return;
                }
                let node = self.new_node(NodeData::Text(RefCell::new(Some(text.into()))));
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
        let NodeData::Element { attrs, name, .. } = &target.node_data else {
            panic!("not an element!");
        };
        let mut attrs = attrs.borrow_mut();

        let existing_names = attrs
            .iter()
            .map(|attr| attr.name().clone())
            .collect::<HashSet<_>>();
        for attr in new_attrs {
            let id = name.parse_attr_id(
                &Prefix::new(attr.name.ns.into(), attr.name.prefix.map(Atom::Prefix)),
                attr.name.local.into(),
            );
            if existing_names.contains(&id) {
                continue;
            }
            attrs.push(Attr::new(id, self.allocator.alloc_str(&attr.value)));
        }
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
fn parse_markup5ever() {
    use crate::attribute::data::{inheritable::Inheritable, presentation::LengthPercentage};
    use lightningcss::{
        properties::svg::SVGPaint,
        values::{
            color::{CssColor, RGBA},
            length::LengthValue,
            percentage::{DimensionPercentage, Percentage},
        },
    };
    let source = r#"<svg version="1.1" baseProfile="full" width="300" height="200" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="black" />
  <circle cx="150" cy="100" r="90" fill="blue" />
  <style>rect { fill: blue; }</style>
</svg>"#;
    let values = Allocator::new_values();
    let mut arena = Allocator::new_arena();
    let mut allocator = Allocator::new(&mut arena, &values);
    let document = parse(source, &mut allocator).find_element().unwrap();

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
        &Attr::Width(LengthPercentage::Percentage(Percentage(1.0)))
    );
    assert_eq!(
        &*attributes.item(1).unwrap(),
        &Attr::Height(LengthPercentage::Percentage(Percentage(1.0)))
    );
    assert_eq!(
        &*attributes.item(2).unwrap(),
        &Attr::Fill(Inheritable::Defined(SVGPaint::Color(CssColor::RGBA(
            RGBA {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255
            }
        ))))
    );

    let circle = rect.next_element_sibling().unwrap();
    assert_eq!(circle.qual_name(), &ElementId::Circle);
    let attributes = circle.attributes();
    assert_eq!(attributes.len(), 4);
    assert_eq!(
        &*attributes.item(0).unwrap(),
        &Attr::CX(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(150.0)
        )))
    );
    assert_eq!(
        &*attributes.item(1).unwrap(),
        &Attr::CY(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(100.0)
        )))
    );
    assert_eq!(
        &*attributes.item(2).unwrap(),
        &Attr::RCircle(LengthPercentage(DimensionPercentage::Dimension(
            LengthValue::Px(90.0)
        )))
    );
    assert_eq!(
        &*attributes.item(3).unwrap(),
        &Attr::Fill(Inheritable::Defined(SVGPaint::Color(CssColor::RGBA(
            RGBA {
                red: 0,
                green: 0,
                blue: 255,
                alpha: 255
            }
        ))))
    );
}
