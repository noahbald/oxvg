use std::collections::HashSet;

use oxc_allocator::{Allocator, Box, Vec};

#[cfg(feature = "html")]
use crate::scanner::HTML;
use crate::{
    Error, ErrorKind, Span,
    ast::{
        Attribute, AttributeValue, CData, Comment, Doctype, Document, Element, MimeType, Name,
        Node, ProcessingInstruction, Text, TextSpan,
    },
    entity::Entities,
    error::{ParseResult, SourceError},
    options::{AllowDTD, ParseOptions},
    scanner::{
        MimeCategory, Token, TokenKind, TokenTrait as _, Tokeniser, TokeniserTrait as _, XML,
    },
    span::SPAN,
};

pub struct Parser<'a> {
    allocator: &'a Allocator,
    source: &'a str,
    mime_type: MimeType,
    options: ParseOptions,
}

#[cfg(feature = "html")]
fn is_void_element(name: &str) -> bool {
    [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
        "track", "wbr",
    ]
    .iter()
    .any(|v| name.eq_ignore_ascii_case(v))
}
#[cfg(feature = "html")]
fn is_raw_element(name: &str) -> bool {
    ["script", "style"]
        .iter()
        .any(|v| name.eq_ignore_ascii_case(v))
}
#[cfg(feature = "html")]
fn is_raw_escapable_element(name: &str) -> bool {
    ["textarea", "title"]
        .iter()
        .any(|v| name.eq_ignore_ascii_case(v))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParserState {
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
    Doctype,
}

impl<'a> Parser<'a> {
    pub fn new(allocator: &'a Allocator, source: &'a str, mime_type: MimeType) -> Self {
        Self {
            allocator,
            source,
            mime_type,
            options: ParseOptions::default(),
        }
    }

    pub fn with_options(mut self, options: ParseOptions) -> Self {
        self.options = options;
        self
    }

    pub fn parse(self) -> ParseResult<Document<'a>> {
        match self.mime_type {
            #[cfg(feature = "html")]
            MimeType::TextHTML => parse_html(self)
                .map_err(ErrorKind::SourceError)
                .map_err(|kind| Error { kind, span: SPAN }),
            _ => parse_xml(self),
        }
    }

    pub fn parse_from_string(
        allocator: &'a Allocator,
        source: &'a str,
        mime_type: MimeType,
    ) -> ParseResult<Document<'a>> {
        Self::new(allocator, source, mime_type).parse()
    }

    #[cfg(feature = "html")]
    pub fn parse_html_from_string(
        allocator: &'a Allocator,
        source: &'a str,
    ) -> Result<Document<'a>, SourceError> {
        parse_html(Self::new(allocator, source, MimeType::TextHTML))
    }
}

#[cfg(feature = "html")]
fn parse_html<'a>(parser: Parser<'a>) -> Result<Document<'a>, SourceError> {
    debug_assert_eq!(parser.mime_type, MimeType::TextHTML);
    let Ok(source_len) = u32::try_from(parser.source.len()) else {
        return Err(SourceError::TooLong);
    };

    let allocator = &parser.allocator;
    let mut tokeniser = Tokeniser::<HTML>::new(parser.source);
    let mut document = Document {
        span: Span::new(0, source_len),
        declaration: None,
        doctype: None,
        children: Vec::new_in(&parser.allocator),
        content_type: parser.mime_type,
    };
    let mut stack = vec![];
    let mut element: Option<Element<'a>> = None;
    while !tokeniser.at_end() {
        let token = tokeniser.current_token().kind();
        tokeniser.next();
        let next = tokeniser.current_token().kind();
        match next {
            TokenKind::DocumentStart => unreachable!(),
            TokenKind::TagOpenStart => {
                if let Some(mut element) = element.take() {
                    element.span.end = tokeniser.current_token().span().end;
                    stack.push(element);
                }
                element = Some(Element::proto(
                    tokeniser.current_token().span().start,
                    &parser.allocator,
                ))
            }
            TokenKind::TagName => {
                // EXPECT: `TagName` can only follow `TagOpenStart`
                let element = element.as_mut().unwrap();
                element.name = Name::new(tokeniser.current_token().span(), tokeniser.current_str())
            }
            TokenKind::TagEnd => {
                // EXPECT: `TagEnd` can only follow `TagOpenStart`
                let mut element = element.take().unwrap();
                element.span.end = tokeniser.current_token().span().end;
                stack.push(element);
            }
            TokenKind::StreamEnd => break,
            _ => todo!(),
        }
    }
    Ok(document)
}
fn parse_xml<'a>(parser: Parser<'a>) -> ParseResult<Document<'a>> {
    #[cfg(feature = "html")]
    debug_assert_ne!(parser.mime_type, MimeType::TextHTML);
    let Ok(source_len) = u32::try_from(parser.source.len()) else {
        return Err(Error {
            kind: ErrorKind::SourceError(SourceError::TooLong),
            span: SPAN,
        });
    };

    let allocator = &parser.allocator;
    let mut tokeniser = Tokeniser::<XML>::new(parser.source);
    let mut document = Document {
        span: Span::new(0, source_len),
        declaration: None,
        doctype: None,
        children: Vec::new_in(&parser.allocator),
        content_type: parser.mime_type,
    };

    let mut has_root = false;
    let mut has_doctype = false;
    let mut stack: std::vec::Vec<Element<'a>> = vec![];
    let mut entities = Entities::default();
    let mut state = ParserState::Content;
    let mut node_start = 0;
    let mut name = Name::proto();
    let mut attribute = Attribute::proto();
    let mut scratch = String::new();

    loop {
        let previous = tokeniser.current_token();
        tokeniser.next();
        let token = tokeniser.current_token();
        previous.check_xml_transition(token)?;
        let span = token.span;

        match (token.kind, state) {
            (TokenKind::StreamEnd, _) => break,
            (TokenKind::Text, ParserState::Content) => {
                if stack.is_empty() {
                    if XML::find_non_whitespace(tokeniser.current_slice()).is_some() {
                        return Err(Error::new(ErrorKind::UnexpectedNonWhitespace, span));
                    }
                }
                let text = TextSpan::new(span, tokeniser.current_str());
                let node = Node::Text(Box::new_in(
                    Text::new(
                        span,
                        entities.resolve::<false>(text, &mut scratch, allocator)?,
                    ),
                    allocator,
                ));
                push_node(&mut stack, &mut document, node);
            }
            (TokenKind::TagOpenStart, ParserState::Content) => {
                if stack.is_empty() {
                    if has_root {
                        return Err(Error::new(ErrorKind::DisallowedMultipleRootElements, span));
                    }
                    has_root = true;
                }
                stack.push(Element::proto(span.start, allocator));
                state = ParserState::OpenTag;
            }
            (TokenKind::TagName, ParserState::OpenTag) => {
                // EXPECT: `OpenTag` should push to stack
                let element = stack.last_mut().unwrap();
                element.name = Name::new(span, tokeniser.current_str());
            }
            (TokenKind::AttrName, ParserState::OpenTag) => {
                // EXPECT: `OpenTag` should push to stack
                let element = stack.last_mut().unwrap();
                let name = tokeniser.current_str();
                if element.attributes.iter().any(|a| a.name.value == name) {
                    return Err(Error::new(ErrorKind::DisallowedDuplicateAttributes, span));
                }
                attribute.span.start = span.start;
                attribute.name = Name::new(span, name);
                state = ParserState::Attribute;
            }
            (TokenKind::ValueSingleQuote | TokenKind::ValueDoubleQuote, ParserState::Attribute) => {
                // EXPECT: `OpenTag` -> `Attribute` should push to stack
                let element = stack.last_mut().unwrap();
                let str = tokeniser.current_str().as_str();
                let value_span = Span::new(
                    tokeniser.current_token().span().start + 1,
                    tokeniser.current_token().span().end - 1,
                );
                let value = TextSpan::new(value_span, tokeniser.str(value_span));
                attribute.value = AttributeValue::new(
                    span,
                    entities.resolve::<true>(value, &mut scratch, allocator)?,
                );
                attribute.span.end = span.end;
                let attribute = std::mem::replace(&mut attribute, Attribute::proto());
                element.attributes.push(attribute);
                state = ParserState::OpenTag;
            }
            (TokenKind::TagEnd, ParserState::OpenTag) => state = ParserState::Content,
            (TokenKind::TagCloseStart, ParserState::Content) => {
                if stack.is_empty() {
                    return Err(Error::new(ErrorKind::UnexpectedCloseTag, span));
                }
                state = ParserState::CloseTag;
            }
            (TokenKind::TagName, ParserState::CloseTag) => {
                // EXPECT: `CloseTag` should have checked is_empty
                let element = stack.last_mut().unwrap();
                if element.name.value != tokeniser.current_str() {
                    return Err(Error::new(ErrorKind::MismatchedCloseTag, span));
                }
            }
            (TokenKind::TagEnd, ParserState::CloseTag)
            | (TokenKind::TagSelfEnd, ParserState::OpenTag) => {
                // EXPECT: `CloseTag` should have checked is_empty
                // EXPECT: `OpenTag` should push to stack
                let mut element = stack.pop().unwrap();
                element.span.end = span.end;
                let node = Node::Element(Box::new_in(element, allocator));
                push_node(&mut stack, &mut document, node);
                state = ParserState::Content;
            }
            (TokenKind::PIStart, ParserState::Content) => {
                node_start = span.start;
                state = ParserState::ProcessingInstruction;
            }
            (TokenKind::PITarget, ParserState::ProcessingInstruction) => {
                name = Name::new(span, tokeniser.current_str());
            }
            (TokenKind::PIContentEndInclusive, ParserState::ProcessingInstruction) => {
                let target = std::mem::replace(&mut name, Name::proto());
                let value = text_before_suffix(&tokeniser, "?>");
                let processing_instruction =
                    ProcessingInstruction::new(Span::new(node_start, span.end), target, value);
                let node =
                    Node::ProcessingInstruction(Box::new_in(processing_instruction, allocator));
                push_node(&mut stack, &mut document, node);
                state = ParserState::Content;
            }
            (TokenKind::CommentStart, ParserState::Content) => {
                node_start = span.start;
                state = ParserState::Comment;
            }
            (TokenKind::CommentTextEndInclusive, ParserState::Comment) => {
                let value = text_before_suffix(&tokeniser, "-->");
                let comment = Comment::new(Span::new(node_start, span.end), value);
                let node = Node::Comment(Box::new_in(comment, allocator));
                push_node(&mut stack, &mut document, node);
                state = ParserState::Content;
            }
            (TokenKind::CDataStart, ParserState::Content) => {
                node_start = span.start;
                state = ParserState::CData;
            }
            (TokenKind::CDataTextEndInclusive, ParserState::CData) => {
                let value = text_before_suffix(&tokeniser, "]]>");
                let c_data = CData::new(Span::new(node_start, span.end), value);
                let node = Node::CData(Box::new_in(c_data, allocator));
                push_node(&mut stack, &mut document, node);
                state = ParserState::Content;
            }
            (TokenKind::ValueUnquoted | TokenKind::TextHTMLFallback, _) => {
                return Err(Error::new(ErrorKind::UnrecognisedData, span));
            }
            (TokenKind::DoctypeStart, ParserState::Content) => {
                // Only at document level, before the root element, at most once
                if !stack.is_empty() || has_root || has_doctype {
                    return Err(Error::new(ErrorKind::UnexpectedDoctype, span));
                }
                has_doctype = true;
                if matches!(parser.options.allow_dtd, AllowDTD::False) {
                    return Err(Error::new(
                        ErrorKind::SourceError(SourceError::DTDDetected),
                        span,
                    ));
                }
                node_start = span.start;
                let doctype = doctype::parse::<XML, false>(
                    &tokeniser,
                    &mut entities,
                    &mut scratch,
                    allocator,
                )?;
                tokeniser.force(
                    doctype.span.end as usize,
                    Token {
                        kind: TokenKind::DoctypeContentEndInclusive,
                        span: doctype.span,
                    },
                );
                document.doctype = Some(doctype);
                state = ParserState::Content;
            }
            state => unreachable!("{state:?}"),
        }
    }
    if state != ParserState::Content || !stack.is_empty() {
        Err(Error::new(
            ErrorKind::UnexpectedEndOfFile,
            tokeniser.current_token().span(),
        ))
    } else if !has_root {
        Err(Error::new(
            ErrorKind::MissingRootElement,
            Span::splat(source_len),
        ))
    } else {
        Ok(document)
    }
}
fn push_node<'a>(stack: &mut [Element<'a>], document: &mut Document<'a>, node: Node<'a>) {
    match stack.last_mut() {
        Some(parent) => parent.children.push(node),
        None => document.children.push(node),
    }
}
fn text_before_suffix<'a>(tokeniser: &Tokeniser<'a, XML>, suffix: &str) -> TextSpan<'a> {
    let span = tokeniser.current_token().span();
    let str = tokeniser.current_str().as_str();
    debug_assert!(str.ends_with(suffix));
    let value = str.strip_suffix(suffix).unwrap_or(str);
    TextSpan::new(Span::new(span.start, span.end - suffix.len() as u32), value)
}

mod doctype {
    use oxc_allocator::GetAllocator;

    use crate::{
        Error, ErrorKind,
        ast::{DTDItem, Doctype, EntityDef, ExternalId, Name, NotationDataDeclaration, TextSpan},
        dtd_scanner::{self, TokenKind},
        entity::Entities,
        scanner::{MimeCategory, Tokeniser, TokeniserTrait},
        span::SPAN,
    };

    pub fn parse<'a, M: MimeCategory, const HTML: bool>(
        parent: &Tokeniser<'a, M>,
        entities: &mut Entities<'a, HTML>,
        scratch: &mut String,
        allocator: &impl GetAllocator<'a>,
    ) -> Result<Doctype<'a>, Error> {
        let mut tokeniser = dtd_scanner::Tokeniser::new(parent);
        let mut doctype = Doctype::proto(allocator);
        let mut external_id = ExternalId::proto();
        let mut int_subset = false;
        let mut entity_name = Name::<'a>::proto();
        let mut entity_span = SPAN;

        doctype.span.start = tokeniser.current_token().span.start;

        loop {
            tokeniser.next();
            let token = tokeniser.current_token();
            let span = token.span;

            match token.kind {
                TokenKind::StreamEnd => {
                    return Err(Error::point(ErrorKind::UnexpectedEndOfFile, span.end));
                }
                TokenKind::DoctypeDefinitionStart => unreachable!(),
                TokenKind::Name => doctype.name = Name::new(span, tokeniser.str(span)),
                TokenKind::System | TokenKind::Public => {
                    external_id = ExternalId::proto();
                    external_id.span.start = span.start;
                }
                TokenKind::PubidLiteral => {
                    let value = tokeniser.current_str().dequote();
                    external_id.public = Some(TextSpan::new(span, value));
                }
                TokenKind::SystemLiteral => {
                    let value = tokeniser.current_str().dequote();
                    external_id.system = TextSpan::new(span, value);
                    if !int_subset {
                        doctype.external_id =
                            Some(std::mem::replace(&mut external_id, ExternalId::proto()))
                    } else {
                        doctype.items.push(DTDItem::new_general_entity(
                            entity_span,
                            entity_name,
                            EntityDef::new_external_id(
                                span,
                                std::mem::replace(&mut external_id, ExternalId::proto()),
                                None,
                                allocator,
                            ),
                            allocator,
                        ))
                    }
                }
                TokenKind::IntSubsetStart => int_subset = true,
                TokenKind::IntSubsetNotEntityDecl => doctype.items.push(
                    DTDItem::new_unparsed_dtd_item(span, tokeniser.current_str(), allocator),
                ),
                TokenKind::EntityDeclStart => {
                    entity_span.start = span.start;
                }
                TokenKind::EntityParameterEntityEndInclusive => {
                    entity_span.end = span.end;
                    doctype.items.push(DTDItem::new_unparsed_dtd_item(
                        entity_span,
                        tokeniser.str(entity_span),
                        allocator,
                    ));
                }
                TokenKind::GeneralEntityDeclName => {
                    entity_name = Name::new(span, tokeniser.current_str())
                }
                TokenKind::GeneralEntityValue => {
                    entity_span.end = span.end;
                    let raw = TextSpan::new(span, tokeniser.current_str().dequote());
                    let value = entities.resolve::<false>(raw, scratch, allocator)?;
                    entities.register(entity_name.value.as_str(), value.escaped.as_str());
                    doctype.items.push(DTDItem::new_general_entity(
                        entity_span,
                        entity_name,
                        EntityDef::new_entity_value(span, value, allocator),
                        allocator,
                    ))
                }
                TokenKind::GeneralEntityNotationData => {
                    if let Some(DTDItem::GeneralEntity(entity)) = doctype.items.last_mut() {
                        if let EntityDef::ExternalEntityDeclaration(decl) = &mut entity.body {
                            decl.notation_data_declaration = Some(NotationDataDeclaration::new(
                                span,
                                Name::new(span, tokeniser.current_str()),
                            ))
                        }
                    }
                }
                TokenKind::IntSubsetEnd => {}
                TokenKind::End => {
                    doctype.span.end = span.end;
                    return Ok(doctype);
                }
                TokenKind::InvalidData => {
                    return Err(Error::new(ErrorKind::InvalidDoctypeData, span));
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use oxc_allocator::Allocator;

    use crate::{
        ErrorKind, MimeType, Parser, Span,
        ast::{
            Attribute, AttributeValue, CData, Comment, Name, ProcessingInstruction, Str, TextSpan,
        },
        span::SPAN,
    };

    #[test]
    fn xml_well_formed() {
        let allocator = Allocator::new();
        let document = Parser::parse_from_string(
            &allocator,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- comment -->
<element>
  <child attribute="value" />
  <![CDATA[ cdata ]]>
</element>"#,
            MimeType::ApplicationXML,
        )
        .unwrap();

        assert_eq!(document.span, Span::new(0, 128));
        assert_eq!(document.content_type, MimeType::ApplicationXML);

        let mut children = document.children.into_iter();
        assert_eq!(
            *children
                .next()
                .unwrap()
                .to_processing_instruction()
                .unwrap(),
            ProcessingInstruction {
                span: Span::new(0, 38),
                target: Name {
                    span: Span::new(2, 5),
                    value: Str::new("xml")
                },
                value: TextSpan {
                    span: Span::new(6, 36),
                    value: Str::new(r#"version="1.0" encoding="UTF-8""#)
                }
            }
        );
        assert_eq!(
            *children.next().unwrap().to_text().unwrap(),
            TextSpan::new(Span::new(38, 39), "\n")
        );
        assert_eq!(
            *children.next().unwrap().to_comment().unwrap(),
            Comment {
                span: Span::new(39, 55),
                value: TextSpan::new(Span::new(43, 52), " comment ")
            }
        );
        assert_eq!(
            *children.next().unwrap().to_text().unwrap(),
            TextSpan {
                span: Span::new(55, 56),
                value: Str::new("\n")
            }
        );

        let element = children.next().unwrap().to_element().unwrap();
        assert_eq!(element.span, Span::new(56, 128));
        assert_eq!(
            element.name,
            Name {
                span: Span::new(57, 64),
                value: Str::new("element")
            }
        );
        assert_eq!(element.attributes.len(), 0);
        let mut element_children = element.unbox().children.into_iter();
        assert_eq!(
            *element_children.next().unwrap().to_text().unwrap(),
            TextSpan {
                span: Span::new(65, 68),
                value: Str::new("\n  ")
            }
        );

        let child = element_children.next().unwrap().to_element().unwrap();
        assert_eq!(child.span, Span::new(68, 95));
        assert_eq!(
            child.name,
            Name {
                span: Span::new(69, 74),
                value: Str::new("child")
            }
        );
        assert_eq!(child.children.len(), 0);
        let mut child_attributes = child.unbox().attributes.into_iter();
        assert_eq!(
            child_attributes.next().unwrap(),
            Attribute {
                span: Span::new(75, 92),
                name: Name {
                    span: Span::new(75, 84),
                    value: Str::new("attribute"),
                },
                value: AttributeValue {
                    span: Span::new(85, 92),
                    value: Str::new("value")
                }
            }
        );
        assert!(child_attributes.next().is_none());

        assert_eq!(
            *dbg!(element_children.next()).unwrap().to_text().unwrap(),
            TextSpan {
                span: Span::new(95, 98),
                value: Str::new("\n  ")
            }
        );
        assert_eq!(
            *element_children.next().unwrap().to_c_data().unwrap(),
            CData {
                span: Span::new(98, 117),
                value: TextSpan::new(Span::new(107, 114), " cdata ")
            }
        );
        assert_eq!(
            *element_children.next().unwrap().to_text().unwrap(),
            TextSpan::new(Span::new(117, 118), "\n")
        );

        assert!(children.next().is_none());
    }

    #[test]
    fn xml_err_multiple_root_elements() {
        let allocator = Allocator::new();
        let error = Parser::parse_from_string(
            &allocator,
            r#"<element/>
<element/>"#,
            MimeType::ApplicationXML,
        )
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::DisallowedMultipleRootElements);
        assert_eq!(error.span, Span::new(11, 12));
    }

    #[test]
    fn xml_err_duplicate_attributes() {
        let allocator = Allocator::new();
        let error = Parser::parse_from_string(
            &allocator,
            r#"<element foo="bar" foo="bar"/>"#,
            MimeType::ApplicationXML,
        )
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::DisallowedDuplicateAttributes);
        assert_eq!(error.span, Span::new(19, 22));
    }
}
