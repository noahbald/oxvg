use oxc_allocator::{Box, GetAllocator, Vec};

use crate::{Span, simd, span::SPAN};

#[derive(Eq, PartialEq, Debug)]
pub enum MimeType {
    #[cfg(feature = "html")]
    TextHTML,
    TextXML,
    ApplicationXML,
    ApplicationXHTMLPlusXML,
    ImageSVGPlusXML,
}

#[derive(Debug)]
pub struct Document<'a> {
    pub span: Span,
    pub declaration: Option<XMLDeclaration<'a>>,
    pub doctype: Option<Doctype<'a>>,
    pub children: Vec<'a, Node<'a>>,
    pub content_type: MimeType,
}

#[derive(Debug)]
pub enum Node<'a> {
    Element(Box<'a, Element<'a>>),
    Text(Box<'a, Text<'a>>),
    Comment(Box<'a, Comment<'a>>),
    CData(Box<'a, CData<'a>>),
    ProcessingInstruction(Box<'a, ProcessingInstruction<'a>>),
}

impl<'a> Node<'a> {
    pub fn element(&self) -> Option<&Box<'a, Element<'a>>> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
    pub fn text(&self) -> Option<&Box<'a, Text<'a>>> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn comment(&self) -> Option<&Box<'a, Comment<'a>>> {
        match self {
            Self::Comment(comment) => Some(comment),
            _ => None,
        }
    }

    pub fn c_data(&self) -> Option<&Box<'a, CData<'a>>> {
        match self {
            Self::CData(c_data) => Some(c_data),
            _ => None,
        }
    }

    pub fn processing_instruction(&self) -> Option<&Box<'a, ProcessingInstruction<'a>>> {
        match self {
            Self::ProcessingInstruction(processing_instruction) => Some(processing_instruction),
            _ => None,
        }
    }

    pub fn to_element(self) -> Option<Box<'a, Element<'a>>> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
    pub fn to_text(self) -> Option<Box<'a, Text<'a>>> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn to_comment(self) -> Option<Box<'a, Comment<'a>>> {
        match self {
            Self::Comment(comment) => Some(comment),
            _ => None,
        }
    }

    pub fn to_c_data(self) -> Option<Box<'a, CData<'a>>> {
        match self {
            Self::CData(c_data) => Some(c_data),
            _ => None,
        }
    }

    pub fn to_processing_instruction(self) -> Option<Box<'a, ProcessingInstruction<'a>>> {
        match self {
            Self::ProcessingInstruction(processing_instruction) => Some(processing_instruction),
            _ => None,
        }
    }

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
pub struct Element<'a> {
    pub span: Span,
    pub name: Name<'a>,
    pub attributes: Vec<'a, Attribute<'a>>,
    pub children: Vec<'a, Node<'a>>,
}

impl<'a> Element<'a> {
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

    pub fn empty(span: Span, name: Name<'a>, allocator: &impl GetAllocator<'a>) -> Self {
        Self::new(span, name, Vec::new_in(allocator), Vec::new_in(allocator))
    }

    pub(crate) fn proto(start: u32, allocator: &impl GetAllocator<'a>) -> Self {
        Self::empty(Span::splat(start), Name::proto(), allocator)
    }
}

#[derive(Debug, PartialEq)]
pub struct Name<'a> {
    pub span: Span,
    pub value: Str<'a>,
}

impl<'a> Name<'a> {
    pub fn new(span: Span, value: impl Into<Str<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    pub fn empty(span: Span) -> Self {
        Self::new(span, Str::empty())
    }

    pub fn proto() -> Self {
        Self::empty(SPAN)
    }

    pub fn is_name<const HTML: bool, const Attribute: bool>(name: &str) -> bool {
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            return false;
        }
        if HTML {
            simd::is_html_name_start(bytes)
                && if Attribute {
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
pub struct Attribute<'a> {
    pub span: Span,
    pub name: Name<'a>,
    pub value: AttributeValue<'a>,
}

impl<'a> Attribute<'a> {
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
pub struct AttributeValue<'a> {
    pub span: Span,
    pub value: EscapedStr<'a>,
}

impl<'a> AttributeValue<'a> {
    pub fn new(span: Span, value: impl Into<EscapedStr<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    pub fn proto() -> Self {
        Self {
            span: SPAN,
            value: EscapedStr::empty(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Text<'a> {
    pub span: Span,
    pub value: EscapedStr<'a>,
}

impl<'a> Text<'a> {
    pub fn new(span: Span, value: impl Into<EscapedStr<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    pub fn empty(span: Span) -> Self {
        Self::new(span, EscapedStr::empty())
    }

    pub fn proto() -> Self {
        Self::empty(SPAN)
    }
}

#[derive(Debug, PartialEq)]
/// A span of text, not including any quotes quotes or other AST decoration.
pub struct TextSpan<'a> {
    pub span: Span,
    pub value: Str<'a>,
}

impl<'a> TextSpan<'a> {
    pub fn new(span: Span, value: impl Into<Str<'a>>) -> Self {
        Self {
            span,
            value: value.into(),
        }
    }

    pub fn empty(span: Span) -> Self {
        Self::new(span, Str::empty())
    }

    pub fn proto() -> Self {
        Self::empty(SPAN)
    }
}

#[derive(Debug, PartialEq)]
pub struct Comment<'a> {
    pub span: Span,
    pub value: TextSpan<'a>,
}

impl<'a> Comment<'a> {
    pub fn new(span: Span, value: TextSpan<'a>) -> Self {
        Self { span, value }
    }

    pub(crate) fn proto(start: u32) -> Self {
        Self::new(Span::splat(start), TextSpan::proto())
    }
}

#[derive(Debug, PartialEq)]
pub struct XMLDeclaration<'a> {
    pub span: Span,
    /// Always `"1.*"`. Unused in parsing.
    pub version: TextSpan<'a>,
    /// Always `"UTF-8"`.
    pub encoding: Span,
    /// Standalone constrains. Unused in parsing.
    pub standalone: Option<TextSpan<'a>>,
}

#[derive(Debug)]
pub struct Doctype<'a> {
    pub span: Span,
    pub name: Name<'a>,
    pub external_id: Option<ExternalId<'a>>,
    pub items: Vec<'a, DTDItem<'a>>,
}

impl<'a> Doctype<'a> {
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

    pub fn proto(allocator: &impl GetAllocator<'a>) -> Self {
        Self {
            span: SPAN,
            name: Name::proto(),
            external_id: None,
            items: Vec::new_in(allocator),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ExternalId<'a> {
    pub span: Span,
    pub public: Option<TextSpan<'a>>,
    pub system: TextSpan<'a>,
}

impl<'a> ExternalId<'a> {
    pub fn new_system(span: Span, system: TextSpan<'a>) -> Self {
        Self {
            span,
            public: None,
            system,
        }
    }

    pub fn new_public(span: Span, public: TextSpan<'a>, system: TextSpan<'a>) -> Self {
        Self {
            span,
            public: Some(public),
            system,
        }
    }

    pub fn proto() -> Self {
        Self {
            span: SPAN,
            public: None,
            system: TextSpan::proto(),
        }
    }
}

#[derive(Debug)]
pub enum DTDItem<'a> {
    UparsedDTDItem(Box<'a, UnparsedDTDItem<'a>>),
    GeneralEntity(Box<'a, GeneralEntity<'a>>),
}

impl<'a> DTDItem<'a> {
    pub fn new_unparsed_dtd_item(
        span: Span,
        raw: Str<'a>,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::UparsedDTDItem(Box::new_in(UnparsedDTDItem::new(span, raw), allocator))
    }

    pub fn new_general_entity(
        span: Span,
        name: Name<'a>,
        body: EntityDef<'a>,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::GeneralEntity(Box::new_in(GeneralEntity::new(span, name, body), allocator))
    }
}

#[derive(Debug, PartialEq)]
pub struct UnparsedDTDItem<'a> {
    pub span: Span,
    pub raw: Str<'a>,
}

impl<'a> UnparsedDTDItem<'a> {
    pub fn new(span: Span, raw: Str<'a>) -> Self {
        Self { span, raw }
    }

    pub fn proto() -> Self {
        Self {
            span: SPAN,
            raw: Str::empty(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DTDDeclKind {
    /// `<!ELEMENT ...>`
    Element,
    /// `<!ATTLIST ...>`
    AttList,
    /// `<!ENTITY % ...>`
    ParameterEntity,
    /// `<!NOTATION ...>`
    Notation,
}

#[derive(Debug)]
pub struct GeneralEntity<'a> {
    pub span: Span,
    pub name: Name<'a>,
    pub body: EntityDef<'a>,
}

impl<'a> GeneralEntity<'a> {
    pub fn new(span: Span, name: Name<'a>, body: EntityDef<'a>) -> Self {
        Self { span, name, body }
    }
}

#[derive(Debug)]
pub enum EntityDef<'a> {
    EntityValue(Box<'a, EntityValue<'a>>),
    ExternalEntityDeclaration(Box<'a, ExternalEntityDeclaration<'a>>),
}

impl<'a> EntityDef<'a> {
    pub fn new_entity_value(
        span: Span,
        value: EscapedStr<'a>,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::EntityValue(Box::new_in(EntityValue::new(span, value), allocator))
    }

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
}

#[derive(Debug, PartialEq)]
pub struct EntityValue<'a> {
    span: Span,
    value: EscapedStr<'a>,
}

impl<'a> EntityValue<'a> {
    pub fn new(span: Span, value: EscapedStr<'a>) -> Self {
        Self { span, value }
    }
}

#[derive(Debug, PartialEq)]
pub struct ExternalEntityDeclaration<'a> {
    pub span: Span,
    pub external_id: ExternalId<'a>,
    pub notation_data_declaration: Option<NotationDataDeclaration<'a>>,
}

impl<'a> ExternalEntityDeclaration<'a> {
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

pub enum ExternalID<'a> {
    System(Box<'a, SystemExternalID<'a>>),
    Public(Box<'a, PublicExternalID<'a>>),
}

pub struct SystemExternalID<'a> {
    span: Span,
    literal: Text<'a>,
}

pub struct PublicExternalID<'a> {
    span: Span,
    pubid_literal: Text<'a>,
    system_literal: Text<'a>,
}

#[derive(Debug, PartialEq)]
pub struct NotationDataDeclaration<'a> {
    span: Span,
    name: Name<'a>,
}

impl<'a> NotationDataDeclaration<'a> {
    pub fn new(span: Span, name: Name<'a>) -> Self {
        Self { span, name }
    }
}

#[derive(Debug, PartialEq)]
pub struct CData<'a> {
    pub span: Span,
    pub value: TextSpan<'a>,
}

impl<'a> CData<'a> {
    pub fn new(span: Span, value: TextSpan<'a>) -> Self {
        Self { span, value }
    }

    pub(crate) fn proto(start: u32) -> Self {
        Self::new(Span::splat(start), TextSpan::proto())
    }
}

#[derive(Debug, PartialEq)]
pub struct ProcessingInstruction<'a> {
    pub span: Span,
    pub target: Name<'a>,
    pub value: TextSpan<'a>,
}

impl<'a> ProcessingInstruction<'a> {
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
pub struct Str<'a>(&'a str);
impl Str<'static> {
    pub const fn empty() -> Self {
        Self::new("")
    }
}
impl<'a> Str<'a> {
    pub const fn new(str: &'a str) -> Self {
        Self(str)
    }

    pub fn as_str(&self) -> &'a str {
        self.0
    }

    fn new_in(str: &str, allocator: &impl GetAllocator<'a>) -> Self {
        Self(allocator.allocator().alloc_str(str))
    }

    pub(crate) fn trim_end(&self, pat: &str) -> Self {
        Self(self.as_str().trim_end_matches(pat))
    }

    pub(crate) fn dequote(self) -> Self {
        let str = self.as_str();
        assert!(str.len() > 1);
        assert!(matches!(str.chars().next(), Some('\'' | '"')));
        assert_eq!(str.chars().next(), str.chars().last());
        Self(&str[1..str.len() - 1])
    }
}
impl<'a> From<&'a str> for Str<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct EscapedStr<'a> {
    pub raw: Str<'a>,
    pub escaped: Str<'a>,
}
impl EscapedStr<'static> {
    pub const fn empty() -> Self {
        Self {
            raw: Str::empty(),
            escaped: Str::empty(),
        }
    }
}
impl<'a> EscapedStr<'a> {
    pub fn new(raw: impl Into<Str<'a>>, escaped: impl Into<Str<'a>>) -> Self {
        Self {
            raw: raw.into(),
            escaped: escaped.into(),
        }
    }

    pub fn as_str(&self) -> &'a str {
        self.escaped.as_str()
    }

    pub fn new_in(
        raw: impl Into<Str<'a>>,
        escaped: &str,
        allocator: &impl GetAllocator<'a>,
    ) -> Self {
        Self::new(raw, allocator.allocator().alloc_str(escaped))
    }
}
