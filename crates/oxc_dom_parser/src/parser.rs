use oxc_allocator::Allocator;
use oxc_str::Str;

use crate::{
    Error, ErrorKind, MimeType, Span,
    ast::{Document, Element, Node, TextSpan},
    error::{ParseResult, SourceError},
    options::ParseOptions,
    scanner::{TokenTrait as _, Tokeniser, TokeniserTrait as _, XML},
    span::SPAN,
};

/// Parser for HTML and XML.
///
/// See [Parser::parse_from_string] for entry function.
pub struct Parser<'a> {
    pub(crate) allocator: &'a Allocator,
    pub(crate) source: &'a str,
    pub(crate) mime_type: MimeType,
    pub(crate) options: ParseOptions,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParserState {
    /// Ready to start a new node
    Content,
    /// Ready to close or add a new attribute
    /// `<element ...`
    OpenTag,
    /// Ready to add attribute value
    /// `<element attribute=...`
    Attribute,
    /// Ready to close
    /// `</element...`
    CloseTag,
    ProcessingInstruction,
    Comment,
    CData,
}

impl<'a> Parser<'a> {
    /// Create a new [Parser].
    pub fn new(allocator: &'a Allocator, source: &'a str, mime_type: MimeType) -> Self {
        Self {
            allocator,
            source,
            mime_type,
            options: ParseOptions::default(),
        }
    }

    /// Set parse options.
    pub fn with_options(mut self, options: ParseOptions) -> Self {
        self.options = options;
        self
    }

    /// Main entry point.
    ///
    /// Returns a result with a document or irrecoverable error.
    ///
    /// Most errors are self-recovered when parsing HTML. Consider using `Parser::parse_from_html_string`
    /// for a reduced error-set.
    pub fn parse(self) -> ParseResult<Document<'a>> {
        match self.mime_type {
            #[cfg(feature = "html")]
            MimeType::TextHTML => crate::parser_html::parse(self)
                .map_err(ErrorKind::SourceError)
                .map_err(|kind| Error { kind, span: SPAN }),
            _ => crate::parser_xml::parse(self),
        }
    }

    /// Parses directly from a string.
    pub fn parse_from_string(
        allocator: &'a Allocator,
        source: &'a str,
        mime_type: MimeType,
    ) -> ParseResult<Document<'a>> {
        Self::new(allocator, source, mime_type).parse()
    }

    /// Parses directly from a string with the given options.
    pub fn parse_from_string_with_options(
        allocator: &'a Allocator,
        source: &'a str,
        mime_type: MimeType,
        options: ParseOptions,
    ) -> ParseResult<Document<'a>> {
        Self::new(allocator, source, mime_type)
            .with_options(options)
            .parse()
    }

    #[cfg(feature = "html")]
    /// Parses HTML directly from a string.
    pub fn parse_html_from_string(
        allocator: &'a Allocator,
        source: &'a str,
    ) -> Result<Document<'a>, SourceError> {
        crate::parser_html::parse(Self::new(allocator, source, MimeType::TextHTML))
    }

    #[cfg(feature = "html")]
    /// Parses HTML directly from a string with the given options.
    pub fn parse_html_from_string_with_options(
        allocator: &'a Allocator,
        source: &'a str,
        options: ParseOptions,
    ) -> Result<Document<'a>, SourceError> {
        crate::parser_html::parse(
            Self::new(allocator, source, MimeType::TextHTML).with_options(options),
        )
    }
}

pub(crate) fn push_node<'a>(
    stack: &mut [Element<'a>],
    document: &mut Document<'a>,
    remaining_nodes_allowed: &mut u32,
    node: Node<'a>,
) -> Result<(), (SourceError, Span)> {
    if *remaining_nodes_allowed == 0 {
        return Err((SourceError::TooManyNodes, node.span()));
    } else {
        *remaining_nodes_allowed -= 1;
    }
    match stack.last_mut() {
        Some(parent) => parent.children.push(node),
        None => document.children.push(node),
    }
    Ok(())
}
pub(crate) fn text_before_suffix<'a>(tokeniser: &Tokeniser<'a, XML>, suffix: &str) -> TextSpan<'a> {
    let span = tokeniser.current_token().span();
    let str = tokeniser.current_str().as_str();
    debug_assert!(str.ends_with(suffix));
    let value = str.strip_suffix(suffix).unwrap_or(str);
    TextSpan::new(Span::new(span.start, span.end - suffix.len() as u32), value)
}
pub(crate) fn dequote<'a>(string: &'a str) -> Str<'a> {
    let bytes = string.as_bytes();
    // The tokeniser should guarantee a matching ASCII quote pair.
    debug_assert!(
        bytes.len() >= 2 && matches!(bytes[0], b'\'' | b'"') && bytes[0] == bytes[bytes.len() - 1]
    );
    Str::from(&string[1..string.len() - 1])
}
