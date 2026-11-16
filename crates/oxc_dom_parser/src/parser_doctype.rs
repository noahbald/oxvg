use oxc_allocator::GetAllocator;

use crate::{
    Error, ErrorKind,
    ast::{DTDItem, Doctype, EntityDef, ExternalId, Name, NotationDataDeclaration, TextSpan},
    dtd_scanner::{self, TokenKind},
    entity::Entities,
    options::{AllowDTD, ParseOptions},
    parser::dequote,
    scanner::{MimeCategory, Tokeniser, TokeniserTrait},
    span::SPAN,
};

pub fn parse<'a, M: MimeCategory, const HTML: bool>(
    parent: &Tokeniser<'a, M>,
    entities: &mut Entities<'a, HTML>,
    scratch: &mut String,
    options: &ParseOptions,
    allocator: &impl GetAllocator<'a>,
) -> Result<Doctype<'a>, Error> {
    let mut tokeniser = dtd_scanner::Tokeniser::new(parent);
    let mut doctype = Doctype::proto(allocator);
    let mut external_id = ExternalId::proto();
    let mut int_subset_start = false;
    let mut int_subset_end = false;
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
                let value = dequote(tokeniser.current_str().as_str());
                external_id.public = Some(TextSpan::new(span, value));
            }
            TokenKind::SystemLiteral => {
                let value = dequote(tokeniser.current_str().as_str());
                external_id.system = TextSpan::new(span, value);
                external_id.span.end = span.end;
                if !int_subset_start {
                    doctype.external_id =
                        Some(std::mem::replace(&mut external_id, ExternalId::proto()))
                } else {
                    doctype.items.push(DTDItem::new_general_entity(
                        entity_span,
                        std::mem::replace(&mut entity_name, Name::proto()),
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
            TokenKind::IntSubsetStart => int_subset_start = true,
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
                let AllowDTD::True(options) = &options.allow_dtd else {
                    unreachable!();
                };
                let raw = TextSpan::new(span, dequote(tokeniser.current_str().as_str()));
                let value = entities.resolve::<false>(raw, scratch, allocator)?;
                entities
                    .register(entity_name.value.as_str(), value.escaped.as_str(), options)
                    .map_err(|kind| Error::new(kind, span))?;
                doctype.items.push(DTDItem::new_general_entity(
                    entity_span,
                    std::mem::replace(&mut entity_name, Name::proto()),
                    EntityDef::new_entity_value(span, value, allocator),
                    allocator,
                ))
            }
            TokenKind::GeneralEntityNotationData => {
                if !int_subset_start {
                    return Err(Error::new(ErrorKind::InvalidDoctypeData, span));
                }
                if let Some(DTDItem::GeneralEntity(entity)) = doctype.items.last_mut() {
                    if let EntityDef::ExternalEntityDeclaration(decl) = &mut entity.body {
                        decl.notation_data_declaration = Some(NotationDataDeclaration::new(
                            span,
                            Name::new(span, tokeniser.current_str()),
                        ));
                        decl.span.end = span.end;
                    }
                }
            }
            TokenKind::IntSubsetEnd => int_subset_end = true,
            TokenKind::End => {
                if int_subset_end || !int_subset_start {
                    doctype.span.end = span.end;
                    return Ok(doctype);
                } else if let Some(DTDItem::GeneralEntity(entity)) = doctype.items.last_mut() {
                    entity.span.end = span.end;
                }
            }
            TokenKind::InvalidData => {
                return Err(Error::new(ErrorKind::InvalidDoctypeData, span));
            }
        }
    }
}
