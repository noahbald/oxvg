//! XML document traits.
use crate::{
    attribute::{Attr, Attributes},
    element::Element,
    node::Node,
};

/// The document is used as a starting point for an xml tree and as a factory for creating
/// xml nodes.
///
/// [MDN | Document](https://developer.mozilla.org/en-US/docs/Web/API/Document)
pub trait Document<'arena> {
    /// The root element containing all the descendant nodes of a document.
    type Root: Element<'arena>;

    /// The document element is the read-only property of the document and returns the [Element]
    /// that is the root element of the [Document]
    ///
    /// [MDN | documentElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/documentElement)
    fn document_element(&self) -> &Self::Root;

    /// Creates a new attribute node and returns it
    ///
    /// [MDN | createAttribute](https://developer.mozilla.org/en-US/docs/Web/API/Document/createAttribute)
    fn create_attribute<'a>(
        &self,
        name: <<<Self::Root as Element<'arena>>::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> <<Self::Root as Element<'arena>>::Attributes<'a> as Attributes<'a>>::Attribute {
        <<Self::Root as Element<'arena>>::Attributes<'a> as Attributes<'a>>::Attribute::new(
            name,
            Default::default(),
        )
    }

    /// Creates a new CDATA section node and returns it
    ///
    /// [MDN | createCDATASection](https://developer.mozilla.org/en-US/docs/Web/API/Document/createCDATASection)
    fn create_c_data_section(
        &self,
        data: <Self::Root as Node<'arena>>::Atom,
        arena: <Self::Root as Node<'arena>>::Arena,
    ) -> <Self::Root as Node<'arena>>::Child;

    /// Creates the html element specified by `tag_name`
    ///
    /// [MDN | createElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/createElement)
    fn create_element(
        &self,
        tag_name: <Self::Root as Element<'arena>>::Name,
        arena: <Self::Root as Node<'arena>>::Arena,
    ) -> Self::Root;

    /// Generates a new processing instruction node and returns it
    ///
    /// [MDN | createProcessingInstruction](https://developer.mozilla.org/en-US/docs/Web/API/Document/createProcessingInstruction)
    fn create_processing_instruction(
        &self,
        target: <Self::Root as Node<'arena>>::Atom,
        data: <Self::Root as Node<'arena>>::Atom,
        arena: <Self::Root as Node<'arena>>::Arena,
    ) -> <<Self::Root as Node<'arena>>::Child as Node<'arena>>::ParentChild;

    /// Creates a new text node
    ///
    /// [MDN | createTextNode](https://developer.mozilla.org/en-US/docs/Web/API/Document/createTextNode)
    fn create_text_node(
        &self,
        data: <Self::Root as Node<'arena>>::Atom,
        arena: <Self::Root as Node<'arena>>::Arena,
    ) -> <Self::Root as Node<'arena>>::Child;
}
