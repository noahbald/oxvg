//! XML document traits.
use std::cell::{Cell, RefCell};

use crate::{
    arena::Arena,
    atom::Atom,
    attribute::data::{Attr, AttrId},
    element::data::ElementId,
    element::Element,
    node::{Node, NodeData, Ref},
};

#[derive(Clone)]

/// The document is used as a starting point for an xml tree and as a factory for creating
/// xml nodes.
///
/// [MDN | Document](https://developer.mozilla.org/en-US/docs/Web/API/Document)
pub struct Document<'arena>(pub Element<'arena>);

impl<'arena> Document<'arena> {
    /// The document element is the read-only property of the document and returns the [Element]
    /// that is the root element of the [Document]
    ///
    /// [MDN | documentElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/documentElement)
    pub fn document_element(&self) -> &Element<'arena> {
        &self.0
    }

    /// Creates a new attribute node and returns it
    ///
    /// [MDN | createAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Document/createAttribute)
    pub fn create_attribute<'a>(&self, name: AttrId<'arena>) -> Attr<'arena> {
        Attr::new(name, Default::default())
    }

    /// Creates a new CDATA section node and returns it
    ///
    /// [MDN | createCDATASection](https://developer.mozilla.org/en-US/docs/Web/API/Document/createCDATASection)
    pub fn create_c_data_section(&self, data: Atom<'arena>, arena: &Arena<'arena>) -> Ref<'arena> {
        self.create_text_node(data, arena)
    }

    /// Creates the html element specified by `tag_name`
    ///
    /// [MDN | createElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/createElement)
    pub fn create_element(
        &self,
        tag_name: ElementId<'arena>,
        arena: &Arena<'arena>,
    ) -> Element<'arena> {
        Element::new(arena.alloc(Node::new(
            NodeData::Element {
                name: tag_name,
                attrs: RefCell::new(vec![]),
                #[cfg(feature = "selectors")]
                selector_flags: Cell::new(None),
            },
            arena.len(),
        )))
        .expect("created element should be an element")
    }

    /// Generates a new processing instruction node and returns it
    ///
    /// [MDN | createProcessingInstruction](https://developer.mozilla.org/en-US/docs/Web/API/Document/createProcessingInstruction)
    pub fn create_processing_instruction(
        &self,
        target: Atom<'arena>,
        data: Atom<'arena>,
        arena: &Arena<'arena>,
    ) -> Ref<'arena> {
        arena.alloc(Node::new(
            NodeData::PI {
                target,
                value: RefCell::new(Some(data)),
            },
            arena.len(),
        ))
    }

    /// Creates a new text node
    ///
    /// [MDN | createTextNode](https://developer.mozilla.org/en-US/docs/Web/API/Document/createTextNode)
    pub fn create_text_node(&self, data: Atom<'arena>, arena: Arena<'arena>) -> Ref<'arena> {
        arena.alloc(Node::new(
            NodeData::Text(RefCell::new(Some(data))),
            arena.len(),
        ))
    }
}
