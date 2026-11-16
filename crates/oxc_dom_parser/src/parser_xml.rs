use oxc_allocator::{Box, Vec};

use crate::{
    Error, ErrorKind, MimeType, Parser, Span,
    ast::{
        Attribute, AttributeValue, CData, Comment, Document, Element, Name, Node,
        ProcessingInstruction, Text, TextSpan,
    },
    entity::Entities,
    error::{ParseResult, SourceError},
    options::AllowDTD,
    parser::{ParserState, push_node, text_before_suffix},
    scanner::{
        MimeCategory as _, Token, TokenKind, TokenTrait as _, Tokeniser, TokeniserTrait as _, XML,
    },
    simd::find_non_xml_char,
    span::SPAN,
};

pub fn parse<'a>(parser: Parser<'a>) -> ParseResult<Document<'a>> {
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
    if let Some(pos) = find_non_xml_char(tokeniser.src) {
        return Err(Error::new(
            ErrorKind::NonXMLCharacter,
            Span::splat(pos as u32),
        ));
    }
    let mut document = Document {
        span: Span::new(0, source_len),
        declaration: None,
        doctype: None,
        children: Vec::new_in(&parser.allocator),
        content_type: parser.mime_type,
    };

    let mut has_root = false;
    let mut has_doctype = false;
    let mut remaining_nodes_allowed = parser.options.max_nodes;
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
                push_node(
                    &mut stack,
                    &mut document,
                    &mut remaining_nodes_allowed,
                    node,
                )
                .map_err(|(err, span)| Error::new(ErrorKind::SourceError(err), span))?;
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
                if previous.span.end == span.start {
                    return Err(Error::new(
                        ErrorKind::UnexpectedNonWhitespace,
                        Span::splat(span.start),
                    ));
                }
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
            // `</a b="1">` / `</a/>`: the tokeniser doesn't know it's in a close tag.
            (TokenKind::AttrName | TokenKind::TagSelfEnd, ParserState::CloseTag) => {
                // `</a foo="bar"` or `</a />`
                return Err(Error::new(ErrorKind::UnexpectedCloseTag, span));
            }
            (TokenKind::TagEnd, ParserState::CloseTag)
            | (TokenKind::TagSelfEnd, ParserState::OpenTag) => {
                // EXPECT: `CloseTag` should have checked is_empty
                // EXPECT: `OpenTag` should push to stack
                let mut element = stack.pop().unwrap();
                element.span.end = span.end;
                let node = Node::Element(Box::new_in(element, allocator));
                push_node(
                    &mut stack,
                    &mut document,
                    &mut remaining_nodes_allowed,
                    node,
                )
                .map_err(|(err, span)| Error::new(ErrorKind::SourceError(err), span))?;
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
                push_node(
                    &mut stack,
                    &mut document,
                    &mut remaining_nodes_allowed,
                    node,
                )
                .map_err(|(err, span)| Error::new(ErrorKind::SourceError(err), span))?;
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
                push_node(
                    &mut stack,
                    &mut document,
                    &mut remaining_nodes_allowed,
                    node,
                )
                .map_err(|(err, span)| Error::new(ErrorKind::SourceError(err), span))?;
                state = ParserState::Content;
            }
            (TokenKind::CDataStart, ParserState::Content) => {
                if stack.is_empty() {
                    return Err(Error::new(ErrorKind::UnexpectedNonWhitespace, span));
                }
                node_start = span.start;
                state = ParserState::CData;
            }
            (TokenKind::CDataTextEndInclusive, ParserState::CData) => {
                let value = text_before_suffix(&tokeniser, "]]>");
                let c_data = CData::new(Span::new(node_start, span.end), value);
                let node = Node::CData(Box::new_in(c_data, allocator));
                push_node(
                    &mut stack,
                    &mut document,
                    &mut remaining_nodes_allowed,
                    node,
                )
                .map_err(|(err, span)| Error::new(ErrorKind::SourceError(err), span))?;
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
                    return Err(Error::new(ErrorKind::DTDDetected, span));
                }
                node_start = span.start;
                let doctype = crate::parser_doctype::parse::<XML, false>(
                    &tokeniser,
                    &mut entities,
                    &mut scratch,
                    &parser.options,
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
            // Unreachable states are bugs and should be handled properly.
            // If you reach this state in production, please raise an issue.
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

#[cfg(test)]
mod test {
    use oxc_allocator::Allocator;
    use oxc_str::Str;

    use crate::{
        ErrorKind, MimeType, Parser, Span,
        ast::{
            Attribute, AttributeValue, CData, Comment, EntityValue, EscapedStr,
            ExternalEntityDeclaration, ExternalId, Name, NotationDataDeclaration,
            ProcessingInstruction, Text, TextSpan, UnparsedDTDItem,
        },
        options::ParseOptions,
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
                    value: Str::new_const("xml")
                },
                value: TextSpan {
                    span: Span::new(6, 36),
                    value: Str::new_const(r#"version="1.0" encoding="UTF-8""#)
                }
            }
        );
        assert_eq!(
            *children.next().unwrap().to_text().unwrap(),
            Text::new(Span::new(38, 39), EscapedStr::new("\n", "\n"))
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
            Text::new(Span::new(55, 56), EscapedStr::new("\n", "\n"))
        );

        let element = children.next().unwrap().to_element().unwrap();
        assert_eq!(element.span, Span::new(56, 128));
        assert_eq!(
            element.name,
            Name {
                span: Span::new(57, 64),
                value: Str::new_const("element")
            }
        );
        assert_eq!(element.attributes.len(), 0);
        let mut element_children = element.unbox().children.into_iter();
        assert_eq!(
            *element_children.next().unwrap().to_text().unwrap(),
            Text::new(Span::new(65, 68), EscapedStr::new("\n  ", "\n  "))
        );

        let child = element_children.next().unwrap().to_element().unwrap();
        assert_eq!(child.span, Span::new(68, 95));
        assert_eq!(
            child.name,
            Name {
                span: Span::new(69, 74),
                value: Str::new_const("child")
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
                    value: Str::new_const("attribute"),
                },
                value: AttributeValue {
                    span: Span::new(85, 92),
                    value: EscapedStr::new("value", "value")
                }
            }
        );
        assert!(child_attributes.next().is_none());

        assert_eq!(
            *element_children.next().unwrap().to_text().unwrap(),
            Text::new(Span::new(95, 98), EscapedStr::new("\n  ", "\n  "))
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
            Text::new(Span::new(117, 118), EscapedStr::new("\n", "\n"))
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

    #[test]
    fn xml_doctype_well_formed() {
        let allocator = Allocator::new();
        let document = Parser::new(
            &allocator,
            r#"<!DOCTYPE name SYSTEM "System literal" [<!ELEMENT name ANY><!ATTLIST name name CDATA #REQUIRED> <!ENTITY name "Entity value"> <!ENTITY name SYSTEM "System literal"> <!ENTITY name PUBLIC "Pubid literal" "System literal" NDATA ndataName> <!ENTITY % pename "<!ENTITY space ' '>"> %pename; <!NOTATION name SYSTEM "System literal"> <? name xml?> <!-- comment > -->]>
<root />"#,
            MimeType::ApplicationXML,
        ).with_options(ParseOptions::default().allow_dtd()).parse().unwrap();
        assert_eq!(document.span, Span::new(0, 370));
        assert_eq!(document.content_type, MimeType::ApplicationXML);

        let doctype = document.doctype.unwrap();
        assert_eq!(doctype.span, Span::new(0, 361));
        assert_eq!(doctype.name, Name::new(Span::new(10, 14), "name"));
        assert_eq!(
            doctype.external_id,
            Some(ExternalId::new_system(
                Span::new(15, 38),
                TextSpan::new(Span::new(22, 38), "System literal")
            ))
        );
        let mut items = doctype.items.into_iter();
        assert_eq!(
            items
                .next()
                .unwrap()
                .to_unparsed_dtd_item()
                .unwrap()
                .unbox(),
            UnparsedDTDItem::new(Span::new(40, 59), "<!ELEMENT name ANY>")
        );
        assert_eq!(
            items
                .next()
                .unwrap()
                .to_unparsed_dtd_item()
                .unwrap()
                .unbox(),
            UnparsedDTDItem::new(Span::new(59, 95), "<!ATTLIST name name CDATA #REQUIRED>")
        );
        let general_entity = items.next().unwrap().to_general_entity().unwrap().unbox();
        assert_eq!(general_entity.span, Span::new(96, 125));
        assert_eq!(general_entity.name, Name::new(Span::new(105, 109), "name"));
        assert_eq!(
            general_entity.body.to_entity_value().unwrap().unbox(),
            EntityValue::new(
                Span::new(110, 124),
                EscapedStr::new("Entity value", "Entity value")
            )
        );
        let general_entity = items.next().unwrap().to_general_entity().unwrap().unbox();
        assert_eq!(general_entity.span, Span::new(126, 164));
        assert_eq!(general_entity.name, Name::new(Span::new(135, 139), "name"));
        assert_eq!(
            general_entity
                .body
                .to_external_entity_declaration()
                .unwrap()
                .unbox(),
            ExternalEntityDeclaration::new(
                Span::new(147, 163),
                ExternalId::new_system(
                    Span::new(140, 163),
                    TextSpan::new(Span::new(147, 163), "System literal")
                ),
                None
            )
        );
        let general_entity = items.next().unwrap().to_general_entity().unwrap().unbox();
        assert_eq!(general_entity.span, Span::new(165, 235));
        assert_eq!(general_entity.name, Name::new(Span::new(174, 178), "name"));
        assert_eq!(
            general_entity
                .body
                .to_external_entity_declaration()
                .unwrap()
                .unbox(),
            ExternalEntityDeclaration::new(
                Span::new(202, 234),
                ExternalId::new_public(
                    Span::new(179, 218),
                    TextSpan::new(Span::new(186, 201), "Pubid literal"),
                    TextSpan::new(Span::new(202, 218), "System literal")
                ),
                Some(NotationDataDeclaration::new(
                    Span::new(225, 234),
                    Name::new(Span::new(225, 234), "ndataName")
                ))
            )
        );
        assert_eq!(
            items
                .next()
                .unwrap()
                .to_unparsed_dtd_item()
                .unwrap()
                .unbox(),
            UnparsedDTDItem::new(
                Span::new(236, 276),
                r#"<!ENTITY % pename "<!ENTITY space ' '>">"#
            )
        );
        assert_eq!(
            items
                .next()
                .unwrap()
                .to_unparsed_dtd_item()
                .unwrap()
                .unbox(),
            UnparsedDTDItem::new(Span::new(277, 285), "%pename;")
        );
        assert_eq!(
            items
                .next()
                .unwrap()
                .to_unparsed_dtd_item()
                .unwrap()
                .unbox(),
            UnparsedDTDItem::new(
                Span::new(286, 326),
                r#"<!NOTATION name SYSTEM "System literal">"#
            )
        );
        assert_eq!(
            items
                .next()
                .unwrap()
                .to_unparsed_dtd_item()
                .unwrap()
                .unbox(),
            UnparsedDTDItem::new(Span::new(327, 340), "<? name xml?>")
        );
        assert_eq!(
            items
                .next()
                .unwrap()
                .to_unparsed_dtd_item()
                .unwrap()
                .unbox(),
            UnparsedDTDItem::new(Span::new(341, 359), "<!-- comment > -->")
        );
        assert!(items.next().is_none());

        let mut children = document.children.into_iter();
        assert_eq!(
            *children.next().unwrap().to_text().unwrap(),
            Text::new(Span::new(361, 362), EscapedStr::new("\n", "\n"))
        );

        let element = children.next().unwrap().to_element().unwrap();
        assert_eq!(element.span, Span::new(362, 370));
        assert_eq!(
            element.name,
            Name {
                span: Span::new(363, 367),
                value: Str::new_const("root")
            }
        );
        assert_eq!(element.attributes.len(), 0);
        assert_eq!(element.children.len(), 0);

        assert!(children.next().is_none());
    }
}
