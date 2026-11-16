use oxc_allocator::{Box, GetAllocator, Vec};
use oxc_str::Str;

use crate::{Span, simd, span::SPAN};

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
/// Specifies whether the XML parser or the HTML parser is used.
pub enum MimeType {
    #[cfg(feature = "html")]
    /// `text/html` parses the document as HTML
    TextHTML,
    /// `text/xml` parses the document as XML
    TextXML,
    /// `application/xml` parses the document as XML
    ApplicationXML,
    /// `application/xhtml+xml` parses the document as XML
    ApplicationXHTMLPlusXML,
    /// `image/svg+xml` parses the document as XML
    ImageSVGPlusXML,
}

#[derive(Debug)]
/// Represents the root of the HTML/XML syntax tree, containing the root declarations and element.
pub struct Document<'a> {
    /// The span of the document.
    pub span: Span,
    /// The XML declaration of an XML document.
    pub declaration: Option<XMLDeclaration<'a>>,
    /// The doctype declaration of the document.
    pub doctype: Option<Doctype<'a>>,
    /// The root children, including comments, processing-instructions, and the root element.
    pub children: Vec<'a, Node<'a>>,
    /// The mime-type used by the parser.
    pub content_type: MimeType,
}

#[derive(Debug)]
/// A content node of the document.
pub enum Node<'a> {
    /// An element.
    Element(Box<'a, Element<'a>>),
    /// Text content.
    Text(Box<'a, Text<'a>>),
    /// A comment.
    Comment(Box<'a, Comment<'a>>),
    /// A CData section of escaped text content.
    CData(Box<'a, CData<'a>>),
    /// Processing instructions for client applications.
    ProcessingInstruction(Box<'a, ProcessingInstruction<'a>>),
}

impl<'a> Node<'a> {
    /// Returns the node as an element, if matching.
    pub fn element(&self) -> Option<&Box<'a, Element<'a>>> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
    /// Returns the node as text, if matching.
    pub fn text(&self) -> Option<&Box<'a, Text<'a>>> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }
    /// Returns the node as a comment, if matching.
    pub fn comment(&self) -> Option<&Box<'a, Comment<'a>>> {
        match self {
            Self::Comment(comment) => Some(comment),
            _ => None,
        }
    }
    /// Returns the node as c-data, if matching.
    pub fn c_data(&self) -> Option<&Box<'a, CData<'a>>> {
        match self {
            Self::CData(c_data) => Some(c_data),
            _ => None,
        }
    }
    /// Returns the node as a processing instruction, if matching.
    pub fn processing_instruction(&self) -> Option<&Box<'a, ProcessingInstruction<'a>>> {
        match self {
            Self::ProcessingInstruction(processing_instruction) => Some(processing_instruction),
            _ => None,
        }
    }

    /// Returns the node as an element, if matching.
    pub fn to_element(self) -> Option<Box<'a, Element<'a>>> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
    /// Returns the node as text, if matching.
    pub fn to_text(self) -> Option<Box<'a, Text<'a>>> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }
    /// Returns the node as a comment, if matching.
    pub fn to_comment(self) -> Option<Box<'a, Comment<'a>>> {
        match self {
            Self::Comment(comment) => Some(comment),
            _ => None,
        }
    }
    /// Returns the node as c-data, if matching.
    pub fn to_c_data(self) -> Option<Box<'a, CData<'a>>> {
        match self {
            Self::CData(c_data) => Some(c_data),
            _ => None,
        }
    }
    /// Returns the node as a processing instruction, if matching.
    pub fn to_processing_instruction(self) -> Option<Box<'a, ProcessingInstruction<'a>>> {
        match self {
            Self::ProcessingInstruction(processing_instruction) => Some(processing_instruction),
            _ => None,
        }
    }

    /// Returns the contained span of the node.
    pub fn span(&self) -> Span {
        match self {
            Self::Element(n) => n.span,
            Self::Text(n) => n.span,
            Self::Comment(n) => n.span,
            Self::CData(n) => n.span,
            Self::ProcessingInstruction(n) => n.span,
        }
    }
}

#[derive(Debug)]
/// An element.
pub struct Element<'a> {
    /// The span of the element.
    pub span: Span,
    /// The qualified name of the element.
    // TODO: Create `QualifiedName` type with prefix, local, and uri.
    pub name: Name<'a>,
    /// The attributes of the element.
    pub attributes: Vec<'a, Attribute<'a>>,
    /// The child nodes of the element.
    pub children: Vec<'a, Node<'a>>,
}

impl<'a> Element<'a> {
    /// Build an [Element].
    pub fn new(
        span: Span,
        name: Name<'a>,
        attributes: Vec<'a, Attribute<'a>>,
        children: Vec<'a, Node<'a>>,
    ) -> Self {
        Self {
            span,
            name,
            attributes,
            children,
        }
    }

    /// Build an [Element] with no children or attributes.
    pub fn empty(span: Span, name: Name<'a>, allocator: &impl GetAllocator<'a>) -> Self {
        Self::new(span, name, Vec::new_in(allocator), Vec::new_in(allocator))
    }

    pub(crate) fn proto(start: u32, allocator: &impl GetAllocator<'a>) -> Self {
        Self::empty(Span::splat(start), Name::proto(), allocator)
    }
}

#[derive(Debug, PartialEq)]
pub struct QualifiedName<'a> {
    pub span: Span,
    pub value: Str<'a>,
    pub uri: Option<Str<'a>>,
    prefix_len: u32,
}

impl<'a> QualifiedName<'a> {
    pub fn new<const HTML: bool>(
        span: Span,
        value: impl Into<Str<'a>>,
        uri: impl Into<Option<Str<'a>>>,
    ) -> Self {
        let value = value.into();
        let prefix_len = if HTML {
            0
        } else {
            crate::simd::find::<true, _>(value.as_bytes(), [b':']).unwrap_or(0)
        } as u32;
        Self {
            span,
            value,
            uri: uri.into(),
            prefix_len,
        }
    }

    pub fn prefix(&self) -> Option<&'a str> {
        if self.prefix_len == 0 {
            None
        } else {
            Some(&self.value.as_str()[..self.prefix_len as usize])
        }
    }

    pub fn local(&self) -> &'a str {
        if self.prefix_len == 0 {
            self.value.as_str()
        } else {
            &self.value.as_str()[self.prefix_len as usize + 1..]
        }
    }

    pub fn is_valid(&self) -> bool {
        let local = self.local();
        crate::simd::is_xml_name_start(local.as_bytes())
            && crate::simd::find::<true, _>(local.as_bytes(), [b':']).is_none()
    }

    pub(crate) fn proto() -> Self {
        Self {
            span: SPAN,
            value: Str::empty(),
            uri: None,
            prefix_len: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
/// A name.
///
/// Parsers should validate whether it is a valid element/attribute name within XML/HTML.
pub struct Name<'a> {
    /// The span of the name.
    pub span: Span,
    /// The name.
    pub value: Str<'a>,
}

impl<'a> Name<'a> {
    /// Build a [Name].
    pub fn new(span: Span, value: impl Into<Str<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    /// Creates an invalid, empty name
    pub(crate) fn empty(span: Span) -> Self {
        Self::new(span, Str::empty())
    }

    pub(crate) fn proto() -> Self {
        Self::empty(SPAN)
    }

    /// Returns whether the given string is a valid name.
    pub fn is_name<const HTML: bool, const ATTRIBUTE: bool>(name: &str) -> bool {
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            return false;
        }
        if HTML {
            simd::is_html_name_start(bytes)
                && if ATTRIBUTE {
                    simd::find_html_attr_name_end::<true>(&bytes[1..]).is_none()
                } else {
                    simd::find_html_tag_name_end::<true>(&bytes[1..]).is_none()
                }
        } else {
            simd::is_xml_name_start(bytes) && simd::find_xml_name_end::<true>(bytes).is_none()
        }
    }
}

#[derive(Debug, PartialEq)]
/// An element attribute.
pub struct Attribute<'a> {
    /// The span of the attribute.
    pub span: Span,
    /// The qualified name of the attribute.
    pub name: Name<'a>,
    /// The value of the attribute.
    pub value: AttributeValue<'a>,
}

impl<'a> Attribute<'a> {
    /// Build an [`Attribute`].
    pub fn new(span: Span, name: Name<'a>, value: AttributeValue<'a>) -> Self {
        Self { span, name, value }
    }

    pub(crate) fn proto() -> Self {
        Self {
            span: Span::splat(0),
            name: Name::proto(),
            value: AttributeValue::proto(),
        }
    }
}

#[derive(Debug, PartialEq)]
/// An element attribute.
pub struct AttributeValue<'a> {
    /// The span of the attribute.
    pub span: Span,
    /// The value of the attribute.
    pub value: EscapedStr<'a>,
}

impl<'a> AttributeValue<'a> {
    /// Build an [AttributeValue].
    pub fn new(span: Span, value: impl Into<EscapedStr<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    pub(crate) fn proto() -> Self {
        Self {
            span: SPAN,
            value: EscapedStr::empty(),
        }
    }

    /// Returns whether the escaped value contains `"`.
    pub fn contains_double_quot(&self) -> bool {
        crate::simd::find::<true, _>(self.value.escaped.as_str().as_bytes(), [b'"']).is_some()
    }
    /// Returns whether the escaped value contains `'`.
    pub fn contains_single_quot(&self) -> bool {
        crate::simd::find::<true, _>(self.value.escaped.as_str().as_bytes(), [b'\'']).is_some()
    }
    /// Returns whether the escaped value contains `'` or `"`.
    pub fn contains_quot(&self) -> bool {
        crate::simd::find::<true, _>(self.value.escaped.as_str().as_bytes(), [b'\'', b'"'])
            .is_some()
    }
}

#[derive(Debug, PartialEq)]
/// A text node.
pub struct Text<'a> {
    /// The span of the text node.
    pub span: Span,
    /// The text content.
    pub value: EscapedStr<'a>,
}

impl<'a> Text<'a> {
    /// Build a [Text].
    pub fn new(span: Span, value: impl Into<EscapedStr<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    /// Build a text node with no content.
    pub fn empty(span: Span) -> Self {
        Self::new(span, EscapedStr::empty())
    }

    pub(crate) fn proto() -> Self {
        Self::empty(SPAN)
    }
}

#[derive(Debug, PartialEq)]
/// A span of text, not including any quotes quotes or other AST decoration.
pub struct TextSpan<'a> {
    /// The span of the text span.
    pub span: Span,
    /// The text content.
    pub value: Str<'a>,
}

impl<'a> TextSpan<'a> {
    /// Build a [TextSpan].
    pub fn new(span: Span, value: impl Into<Str<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    /// Build a text-span with no content
    pub fn empty(span: Span) -> Self {
        Self::new(span, Str::empty())
    }

    pub(crate) fn proto() -> Self {
        Self::empty(SPAN)
    }
}

#[derive(Debug, PartialEq)]
/// A comment node.
pub struct Comment<'a> {
    /// The span of the comment.
    pub span: Span,
    /// The text content of the comment.
    pub value: TextSpan<'a>,
}

impl<'a> Comment<'a> {
    /// Build a [Comment].
    pub fn new(span: Span, value: TextSpan<'a>) -> Self {
        Self { span, value }
    }

    pub(crate) fn proto(start: u32) -> Self {
        Self::new(Span::splat(start), TextSpan::proto())
    }
}

#[derive(Debug, PartialEq)]
/// An XML declaration in an XML document's root.
///
/// This may be present in HTML documents but should be treated as a comment by the client.
pub struct XMLDeclaration<'a> {
    /// The span of the XML declaration.
    pub span: Span,
    /// Always `"1.*"`. Unused in parsing.
    pub version: TextSpan<'a>,
    /// Always `"UTF-8"`.
    pub encoding: Span,
    /// Standalone constrains. Unused in parsing.
    pub standalone: Option<TextSpan<'a>>,
}

#[derive(Debug)]
/// A doctype declaration.
///
/// Invalid doctypes in HTML should be treated as a comment by the client.
/// A valid doctype in HTML is case-insensitively `<!DOCTYPE html>` or `<!DOCTYPE html SYSTEM "about:legacy-compat">`.
pub struct Doctype<'a> {
    /// The span of the doctype.
    pub span: Span,
    /// The name of the doctype.
    pub name: Name<'a>,
    /// The external-id of the doctype.
    pub external_id: Option<ExternalId<'a>>,
    /// The internal-subset items of the doctype.
    pub items: Vec<'a, DTDItem<'a>>,
}

impl<'a> Doctype<'a> {
    /// Build a [Doctype].
    pub fn new(
        span: Span,
        name: Name<'a>,
        external_id: Option<ExternalId<'a>>,
        items: Vec<'a, DTDItem<'a>>,
    ) -> Self {
        Self {
            span,
            name,
            external_id,
            items,
        }
    }

    pub(crate) fn proto(allocator: &impl GetAllocator<'a>) -> Self {
        Self {
            span: SPAN,
            name: Name::proto(),
            external_id: None,
            items: Vec::new_in(allocator),
        }
    }
}

#[derive(Debug, PartialEq)]
/// A doctype external-id.
pub struct ExternalId<'a> {
    /// The span of the external-id.
    pub span: Span,
    /// The public identifier of the external-id.
    pub public: Option<TextSpan<'a>>,
    /// The system uri of the external-id.
    pub system: TextSpan<'a>,
}

impl<'a> ExternalId<'a> {
    /// Build an [ExternalId] with a system uri.
    pub fn new_system(span: Span, system: TextSpan<'a>) -> Self {
        Self {
            span,
            public: None,
            system,
        }
    }

    /// Build an [ExternalId] with a system uri and public identifier.
    pub fn new_public(span: Span, public: TextSpan<'a>, system: TextSpan<'a>) -> Self {
        Self {
            span,
            public: Some(public),
            system,
        }
    }

    pub(crate) fn proto() -> Self {
        Self {
            span: SPAN,
            public: None,
            system: TextSpan::proto(),
        }
    }
}

#[derive(Debug)]
/// An internal-subset declaration of a doctype.
pub enum DTDItem<'a> {
    /// An item unsupported by the parser. Unsupported items are not validated by the parser.
    UnparsedDTDItem(Box<'a, UnparsedDTDItem<'a>>),
    /// A general entity declaration.
    GeneralEntity(Box<'a, GeneralEntity<'a>>),
}

impl<'a> DTDItem<'a> {
    /// Build a [DTDItem] with an unparsed chunk.
    pub fn new_unparsed_dtd_item(
        span: Span,
        raw: Str<'a>,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::UnparsedDTDItem(Box::new_in(UnparsedDTDItem::new(span, raw), allocator))
    }

    /// Build a [DTDItem] with a general entity declaration.
    pub fn new_general_entity(
        span: Span,
        name: Name<'a>,
        body: EntityDef<'a>,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::GeneralEntity(Box::new_in(GeneralEntity::new(span, name, body), allocator))
    }

    /// Returns the item as a general entity, if matching.
    pub fn to_general_entity(self) -> Option<Box<'a, GeneralEntity<'a>>> {
        match self {
            Self::GeneralEntity(general_entity) => Some(general_entity),
            _ => None,
        }
    }

    /// Returns the item as nn unparsed item, if matching.
    pub fn to_unparsed_dtd_item(self) -> Option<Box<'a, UnparsedDTDItem<'a>>> {
        match self {
            Self::UnparsedDTDItem(unparsed_dtd_item) => Some(unparsed_dtd_item),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
/// An unparsed item unsupported by the parser.
pub struct UnparsedDTDItem<'a> {
    /// The span of the item.
    pub span: Span,
    /// The chunk of xml.
    pub raw: Str<'a>,
}

impl<'a> UnparsedDTDItem<'a> {
    /// Build an [UnparsedDTDItem].
    pub fn new(span: Span, raw: impl Into<Str<'a>>) -> Self {
        Self {
            span,
            raw: raw.into(),
        }
    }

    pub(crate) fn proto() -> Self {
        Self {
            span: SPAN,
            raw: Str::empty(),
        }
    }
}

#[derive(Debug)]
/// A general entity declaration.
pub struct GeneralEntity<'a> {
    /// The span of the declaration.
    pub span: Span,
    /// The name of the entity.
    pub name: Name<'a>,
    /// The body of the entity.
    pub body: EntityDef<'a>,
}

impl<'a> GeneralEntity<'a> {
    /// Build a [GeneralEntity].
    pub fn new(span: Span, name: Name<'a>, body: EntityDef<'a>) -> Self {
        Self { span, name, body }
    }
}

#[derive(Debug)]
/// The definition of the entity's body.
pub enum EntityDef<'a> {
    /// An explicit entity value.
    EntityValue(Box<'a, EntityValue<'a>>),
    /// An external entity value.
    ExternalEntityDeclaration(Box<'a, ExternalEntityDeclaration<'a>>),
}

impl<'a> EntityDef<'a> {
    /// Build an [EntityDef] with a value.
    pub fn new_entity_value(
        span: Span,
        value: EscapedStr<'a>,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::EntityValue(Box::new_in(EntityValue::new(span, value), allocator))
    }

    /// Build an [EntityDef] with an external entity.
    pub fn new_external_id(
        span: Span,
        external_id: ExternalId<'a>,
        notation_data_declaration: Option<NotationDataDeclaration<'a>>,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::ExternalEntityDeclaration(Box::new_in(
            ExternalEntityDeclaration::new(span, external_id, notation_data_declaration),
            allocator,
        ))
    }

    /// Returns the definition as an entity value, if matching.
    pub fn to_entity_value(self) -> Option<Box<'a, EntityValue<'a>>> {
        match self {
            Self::EntityValue(entity_value) => Some(entity_value),
            _ => None,
        }
    }

    /// Returns the definition as an external entity declaration, if matching.
    pub fn to_external_entity_declaration(self) -> Option<Box<'a, ExternalEntityDeclaration<'a>>> {
        match self {
            Self::ExternalEntityDeclaration(external_entity_declaration) => {
                Some(external_entity_declaration)
            }
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
/// The value of a declared entity.
pub struct EntityValue<'a> {
    /// The span of the entity value.
    pub span: Span,
    /// The value of the entity value.
    pub value: EscapedStr<'a>,
}

impl<'a> EntityValue<'a> {
    /// Build an [EntityValue].
    pub fn new(span: Span, value: EscapedStr<'a>) -> Self {
        Self { span, value }
    }
}

#[derive(Debug, PartialEq)]
/// The external declaration of a declared entity.
pub struct ExternalEntityDeclaration<'a> {
    /// The span of the externa entity declaration.
    pub span: Span,
    /// The external ID of the external entity declaration.
    pub external_id: ExternalId<'a>,
    /// The notation data declaration of the eternal entity declaration.
    pub notation_data_declaration: Option<NotationDataDeclaration<'a>>,
}

impl<'a> ExternalEntityDeclaration<'a> {
    /// Build an [ExternalEntityDeclaration]
    pub fn new(
        span: Span,
        external_id: ExternalId<'a>,
        notation_data_declaration: Option<NotationDataDeclaration<'a>>,
    ) -> Self {
        Self {
            span,
            external_id,
            notation_data_declaration,
        }
    }
}

#[derive(Debug, PartialEq)]
/// An NDATA declaration.
pub struct NotationDataDeclaration<'a> {
    /// The span of the declaration.
    pub span: Span,
    /// The name of the notation declaration from the external ID.
    pub name: Name<'a>,
}

impl<'a> NotationDataDeclaration<'a> {
    /// Build a [NotationDataDeclaration]
    pub fn new(span: Span, name: Name<'a>) -> Self {
        Self { span, name }
    }
}

#[derive(Debug, PartialEq)]
/// A character data node.
pub struct CData<'a> {
    /// The span of the c-data section.
    pub span: Span,
    /// The text of the c-data section.
    pub value: TextSpan<'a>,
}

impl<'a> CData<'a> {
    /// Build a [CData].
    pub fn new(span: Span, value: TextSpan<'a>) -> Self {
        Self { span, value }
    }

    pub(crate) fn proto(start: u32) -> Self {
        Self::new(Span::splat(start), TextSpan::proto())
    }
}

#[derive(Debug, PartialEq)]
/// A processing instruction.
pub struct ProcessingInstruction<'a> {
    /// The span of the processing instruction.
    pub span: Span,
    /// The name of the target application.
    pub target: Name<'a>,
    /// The text to be processed by the target application.
    pub value: TextSpan<'a>,
}

impl<'a> ProcessingInstruction<'a> {
    /// Build a [ProcessingInstruction].
    pub fn new(span: Span, target: Name<'a>, value: TextSpan<'a>) -> Self {
        Self {
            span,
            target,
            value,
        }
    }

    pub(crate) fn proto(start: u32) -> Self {
        Self::new(Span::splat(start), Name::proto(), TextSpan::proto())
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
/// A string with escaped character references.
pub struct EscapedStr<'a> {
    /// The original string.
    pub raw: Str<'a>,
    /// The escaped string.
    pub escaped: Str<'a>,
}
impl EscapedStr<'static> {
    /// An escaped string of an empty slice (`""`).
    pub const fn empty() -> Self {
        Self {
            raw: Str::empty(),
            escaped: Str::empty(),
        }
    }
}
impl<'a> EscapedStr<'a> {
    /// Build an [EscapedStr].
    pub fn new(raw: impl Into<Str<'a>>, escaped: impl Into<Str<'a>>) -> Self {
        Self {
            raw: raw.into(),
            escaped: escaped.into(),
        }
    }

    /// The escaped text as a string slice.
    pub fn as_str(&self) -> &'a str {
        self.escaped.as_str()
    }

    /// Build an [EscapedStr] in the given allocator.
    pub fn new_in(
        raw: impl Into<Str<'a>>,
        escaped: &str,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::new(raw, allocator.allocator().alloc_str(escaped))
    }
}
