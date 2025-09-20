//! XML document traits.
use std::cell::RefCell;

use lightningcss::rules::CssRuleList;

use crate::{
    arena::Allocator,
    atom::Atom,
    attribute::data::{Attr, AttrId},
    element::{data::ElementId, Element},
    node::{NodeData, Ref},
};

#[derive(Clone)]

/// The document is used as a starting point for an xml tree and as a factory for creating
/// xml nodes.
///
/// [MDN | Document](https://developer.mozilla.org/en-US/docs/Web/API/Document)
pub struct Document<'input, 'arena>(pub Element<'input, 'arena>);

impl<'input, 'arena> Document<'input, 'arena> {
    /// The document element is the read-only property of the document and returns the [Element]
    /// that is the root element of the [Document]
    ///
    /// [MDN | documentElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/documentElement)
    pub fn document_element(&self) -> &Element<'input, 'arena> {
        &self.0
    }

    /// Creates a new attribute node and returns it
    ///
    /// [MDN | createAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Document/createAttribute)
    pub fn create_attribute<'a>(&self, name: AttrId<'input>) -> Attr<'input> {
        Attr::new(name, Default::default())
    }

    /// Creates a new CDATA section node and returns it
    ///
    /// [MDN | createCDATASection](https://developer.mozilla.org/en-US/docs/Web/API/Document/createCDATASection)
    pub fn create_c_data_section(
        &self,
        data: Atom<'input>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Ref<'input, 'arena> {
        self.create_text_node(data, allocator)
    }

    /// Creates the html element specified by `tag_name`
    ///
    /// [MDN | createElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/createElement)
    pub fn create_element(
        &self,
        tag_name: ElementId<'input>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Element<'input, 'arena> {
        Element::new(allocator.alloc(NodeData::Element {
            name: tag_name,
            attrs: RefCell::new(vec![]),
            #[cfg(feature = "selectors")]
            selector_flags: std::cell::Cell::new(None),
        }))
        .expect("created element should be an element")
    }

    /// Generates a new processing instruction node and returns it
    ///
    /// [MDN | createProcessingInstruction](https://developer.mozilla.org/en-US/docs/Web/API/Document/createProcessingInstruction)
    pub fn create_processing_instruction(
        &self,
        target: Atom<'input>,
        data: Atom<'input>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Ref<'input, 'arena> {
        allocator.alloc(NodeData::PI {
            target,
            value: RefCell::new(Some(data)),
        })
    }

    /// Creates a new text node
    ///
    /// [MDN | createTextNode](https://developer.mozilla.org/en-US/docs/Web/API/Document/createTextNode)
    pub fn create_text_node(
        &self,
        data: Atom<'input>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Ref<'input, 'arena> {
        allocator.alloc(NodeData::Text(RefCell::new(Some(data))))
    }

    /// Creates a new text node for a `<style>` element
    pub fn create_style_node(
        &self,
        data: CssRuleList<'input>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Ref<'input, 'arena> {
        allocator.alloc(NodeData::Style(RefCell::new(data)))
    }
}
