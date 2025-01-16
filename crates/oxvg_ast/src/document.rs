use crate::{
    attribute::{Attr, Attributes},
    element::Element,
    node::Node,
};

pub trait Document {
    type Root: Element;

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
        name: <<<Self::Root as Element>::Attributes<'a> as Attributes<'a>>::Attribute as Attr>::Name,
    ) -> <<Self::Root as Element>::Attributes<'a> as Attributes<'a>>::Attribute;

    /// Creates a new CDATA section node and returns it
    ///
    /// [MDN | createCDATASection](https://developer.mozilla.org/en-US/docs/Web/API/Document/createCDATASection)
    fn create_c_data_section(
        &self,
        data: <Self::Root as Node>::Atom,
    ) -> <<Self::Root as Node>::ParentChild as Node>::Child;

    /// Creates the html element specified by `tag_name`
    ///
    /// [MDN | createElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/createElement)
    fn create_element(&self, tag_name: <Self::Root as Element>::Name) -> Self::Root;

    /// Generates a new processing instruction node and returns it
    ///
    /// [MDN | createProcessingInstruction](https://developer.mozilla.org/en-US/docs/Web/API/Document/createProcessingInstruction)
    fn create_processing_instruction(
        &self,
        target: <Self::Root as Node>::Atom,
        data: <Self::Root as Node>::Atom,
    ) -> <<Self::Root as Node>::Child as Node>::ParentChild;

    /// Creates a new text node
    ///
    /// [MDN | createTextNode](https://developer.mozilla.org/en-US/docs/Web/API/Document/createTextNode)
    fn create_text_node(&self, data: <Self::Root as Node>::Atom) -> <Self::Root as Node>::Child;
}
