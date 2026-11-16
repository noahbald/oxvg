use std::{marker::PhantomData, ops::RangeInclusive};

use crate::{
    Span,
    scanner::{MimeCategory, NameKind, TokenTrait, TokeniserTrait},
};

pub struct Tokeniser<'a, M: MimeCategory> {
    src_str: &'a str,
    src: &'a [u8],
    cursor: usize,
    token: Token,
    marker: PhantomData<M>,
}

impl<'a, M: MimeCategory> TokeniserTrait<'a, M> for Tokeniser<'a, M> {
    type Token = Token;

    fn current_token(&self) -> Self::Token {
        self.token
    }
    fn src_str(&self) -> &'a str {
        self.src_str
    }
    fn src(&self) -> &'a [u8] {
        self.src
    }
    fn cursor(&self) -> usize {
        self.cursor
    }
    fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor
    }
    fn advance(&mut self, by: usize) {
        self.cursor += by;
    }
    fn force(&mut self, cursor: usize, token: Self::Token) {
        self.cursor = cursor;
        self.token = token;
    }
}

impl<'a, M: MimeCategory> Tokeniser<'a, M> {
    pub fn new(parent: &crate::scanner::Tokeniser<'a, M>) -> Self {
        Self {
            src_str: parent.src_str,
            src: parent.src,
            cursor: parent.current_token().span.end as usize,
            token: Token::new(
                TokenKind::DoctypeDefinitionStart,
                parent.current_token().span,
            ),
            marker: PhantomData,
        }
    }

    /// doctypedecl = '<!DOCTYPE' S Name (S ExternalID)? S? ('[' intSubset ']' S?)? '>'
    pub fn next(&mut self) {
        self.token = if self.at_end() {
            Token::at(TokenKind::StreamEnd, self.cursor as u32)
        } else {
            match self.token.kind {
                TokenKind::DoctypeDefinitionStart => self.next_name(),
                TokenKind::Name => self
                    .try_next(Self::next_external_id)
                    .or_else(|| self.try_next(Self::next_int_subset_start))
                    .unwrap_or_else(|| self.next_end()),
                TokenKind::Public => self.next_pubid_literal(),
                TokenKind::System | TokenKind::PubidLiteral => self.next_system_literal(),
                TokenKind::SystemLiteral => self
                    .try_next(Self::next_general_entity_notation_data)
                    .or_else(|| self.try_next(Self::next_int_subset_start))
                    .unwrap_or_else(|| self.next_end()),
                TokenKind::IntSubsetStart
                | TokenKind::IntSubsetNotEntityDecl
                | TokenKind::EntityParameterEntityEndInclusive
                | TokenKind::End => self
                    .try_next(Self::next_entity_decl)
                    .or_else(|| self.try_next(Self::next_int_subset))
                    .unwrap_or_else(|| self.next_int_subset_end()),
                TokenKind::EntityDeclStart => self
                    .try_next(Self::next_parameter_entity_end_inclusive)
                    .unwrap_or_else(|| self.next_general_entity_name()),
                TokenKind::GeneralEntityDeclName => self
                    .try_next(Self::next_external_id)
                    .unwrap_or_else(|| self.next_general_entity_value()),
                TokenKind::GeneralEntityNotationData
                | TokenKind::GeneralEntityValue
                | TokenKind::IntSubsetEnd => self.next_end(),
                TokenKind::InvalidData | TokenKind::StreamEnd => self.token,
            }
        };
    }

    /// Name
    pub fn next_name(&mut self) -> Token {
        let start = self.cursor;
        let kind = if self.next_name_range(NameKind::Element).is_some() {
            TokenKind::Name
        } else {
            TokenKind::InvalidData
        };
        Token::new(kind, Span::new_usize(start, self.cursor))
    }

    /// (S ExternalID)?
    /// ExternalId = 'SYSTEM' S SystemLiteral | 'PUBLIC' S PudibLiteral S SystemLiteral
    /// SystemLiteral = ('"' [^"]* '"') | ("'" [^']* "'")
    /// PubidLiteral = '"' PubidChar* '" | "'" (PubidChar - "'")* "'"
    /// PubidChar = #x20 | #xD | #xA | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]
    fn next_external_id(&mut self) -> Option<Token> {
        self.next_whitespace()?;
        let start = self.cursor;
        let slice = self.slice();
        if slice.starts_with(b"SYSTEM") {
            self.advance(6);
            let end = self.cursor();
            Some(Token::new(TokenKind::System, Span::new_usize(start, end)))
        } else if slice.starts_with(b"PUBLIC") {
            self.advance(6);
            let end = self.cursor();
            Some(Token::new(TokenKind::Public, Span::new_usize(start, end)))
        } else {
            None
        }
    }

    /// S PubidLiteral
    /// PubidLiteral = '"' PubidChar* '" | "'" (PubidChar - "'")* "'"
    /// PubidChar = #x20 | #xD | #xA | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]
    fn next_pubid_literal(&mut self) -> Token {
        self.skip_whitespace();
        let start = self.cursor();
        let Some(quote) = self.current_byte() else {
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        if !matches!(quote, b'\'' | b'"') {
            return Token::at(TokenKind::InvalidData, self.cursor as u32);
        }
        self.advance(1);
        if quote == b'\'' {
            let Some(end) = crate::simd::find_mixed_ne::<true, _, _>(
                self.slice(),
                PUBID_CHARS,
                PUBID_RANGES_EX_QUOTE,
            ) else {
                self.end();
                return Token::at(TokenKind::StreamEnd, self.cursor as u32);
            };
            self.advance(end);
        } else {
            let Some(end) =
                crate::simd::find_mixed_ne::<true, _, _>(self.slice(), PUBID_CHARS, PUBID_RANGES)
            else {
                self.end();
                return Token::at(TokenKind::StreamEnd, self.cursor as u32);
            };
            self.advance(end);
        }
        let Some(end_quote) = self.current_byte() else {
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        if end_quote == quote {
            self.advance(1);
            Token::new(
                TokenKind::PubidLiteral,
                Span::new_usize(start, self.cursor()),
            )
        } else {
            Token::at(TokenKind::InvalidData, self.cursor as u32)
        }
    }

    /// S SystemLiteral
    /// SystemLiteral = ('"' [^"]* '"') | ("'" [^']* "'")
    fn next_system_literal(&mut self) -> Token {
        self.skip_whitespace();
        let start = self.cursor();
        let Some(quote) = self.current_byte() else {
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        if !matches!(quote, b'\'' | b'"') {
            return Token::at(TokenKind::InvalidData, self.cursor as u32);
        }
        self.advance(1);
        let Some(end) = crate::simd::find::<true, _>(self.slice(), [quote]) else {
            self.end();
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        self.advance(end + 1);
        Token::new(
            TokenKind::SystemLiteral,
            Span::new_usize(start, self.cursor()),
        )
    }

    /// S 'NDATA' Name
    fn next_general_entity_notation_data(&mut self) -> Option<Token> {
        self.next_whitespace()?;
        if !self.slice().starts_with(b"NDATA") {
            return None;
        }
        self.advance(5);
        self.next_whitespace()?;
        let start = self.cursor;
        if self.next_name_range(NameKind::Element).is_some() {
            Some(Token::new(
                TokenKind::GeneralEntityNotationData,
                Span::new_usize(start, self.cursor),
            ))
        } else {
            None
        }
    }

    /// '['
    pub fn next_int_subset_start(&mut self) -> Option<Token> {
        self.skip_whitespace();
        let Some(start) = self.current_byte() else {
            return None;
        };
        self.advance(1);
        if matches!(start, b'[') {
            Some(Token::at(TokenKind::IntSubsetStart, self.cursor as u32))
        } else {
            return None;
        }
    }

    fn next_entity_decl(&mut self) -> Option<Token> {
        self.skip_whitespace();
        let start = self.cursor;
        if self.slice().starts_with(b"<!ENTITY") {
            self.advance(8);
            let end = self.cursor;
            self.next_whitespace()?;
            Some(Token::new(
                TokenKind::EntityDeclStart,
                Span::new_usize(start, end),
            ))
        } else {
            None
        }
    }

    fn next_int_subset(&mut self) -> Option<Token> {
        self.skip_whitespace();
        let start_char = self.current_byte()?;
        let start = self.cursor;
        match start_char {
            b'<' => {
                if self.slice().starts_with(b"<!--") {
                    self.advance(4);
                    loop {
                        self.advance(crate::simd::find::<true, _>(self.slice(), [b'-'])?);
                        if self.slice().starts_with(b"-->") {
                            self.advance(3);
                            break;
                        }
                        self.advance(1);
                    }
                } else {
                    let mut quote = 0;
                    loop {
                        self.advance(crate::simd::find::<true, _>(
                            self.slice(),
                            [b'>', b'\'', b'"'],
                        )?);
                        if let Some(byte) = self.current_byte() {
                            self.advance(1);
                            if quote == 0 {
                                if byte == b'>' {
                                    break;
                                } else {
                                    quote = byte;
                                }
                            } else if quote == byte {
                                quote = 0;
                            }
                        }
                    }
                }
                Some(Token::new(
                    TokenKind::IntSubsetNotEntityDecl,
                    Span::new_usize(start, self.cursor),
                ))
            }
            b'%' => {
                self.advance(1);
                self.next_name_range(NameKind::Element)?;
                self.next_byte_expect(b';')?;
                Some(Token::new(
                    TokenKind::IntSubsetNotEntityDecl,
                    Span::new_usize(start, self.cursor),
                ))
            }
            _ => None,
        }
    }

    fn next_int_subset_end(&mut self) -> Token {
        self.skip_whitespace();
        let Some(char) = self.current_byte() else {
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        if char == b']' {
            self.advance(1);
            Token::at(TokenKind::IntSubsetEnd, self.cursor as u32)
        } else {
            Token::at(TokenKind::InvalidData, self.cursor as u32)
        }
    }

    fn next_parameter_entity_end_inclusive(&mut self) -> Option<Token> {
        let start = self.cursor;
        self.next_byte_expect(b'%')?;
        self.next_whitespace()?;
        let mut quote = 0;
        loop {
            self.advance(crate::simd::find::<true, _>(
                self.slice(),
                [b'>', b'\'', b'"'],
            )?);
            if let Some(byte) = self.current_byte() {
                self.advance(1);
                if quote == 0 {
                    if byte == b'>' {
                        break;
                    } else {
                        quote = byte;
                    }
                } else if quote == byte {
                    quote = 0;
                }
            }
        }
        Some(Token::new(
            TokenKind::EntityParameterEntityEndInclusive,
            Span::new_usize(start, self.cursor),
        ))
    }

    fn next_general_entity_name(&mut self) -> Token {
        let mut token = self.next_name();
        if matches!(token.kind, TokenKind::Name) {
            token.kind = TokenKind::GeneralEntityDeclName;
        }
        token
    }

    fn next_general_entity_value(&mut self) -> Token {
        self.skip_whitespace();
        let start = self.cursor;
        let Some(quote) = self.current_byte() else {
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        if !matches!(quote, b'\'' | b'"') {
            return Token::at(TokenKind::InvalidData, self.cursor as u32);
        }
        self.advance(1);
        let Some(end) = crate::simd::find::<true, _>(self.slice(), [quote]) else {
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        self.advance(end + 1);
        return Token::new(
            TokenKind::GeneralEntityValue,
            Span::new_usize(start, self.cursor),
        );
    }

    /// '>'
    pub fn next_end(&mut self) -> Token {
        self.skip_whitespace();
        let Some(end) = self.current_byte() else {
            return Token::at(TokenKind::StreamEnd, self.cursor as u32);
        };
        if matches!(end, b'>') {
            self.advance(1);
            Token::at(TokenKind::End, self.cursor as u32)
        } else {
            Token::at(TokenKind::InvalidData, self.cursor as u32)
        }
    }
}

/// PubidChar = ... | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]
/// [#..%] | ['../..0..9..;] | [?..@..A..Z]
const PUBID_RANGES: [RangeInclusive<u8>; 4] = [b'#'..=b'%', b'\''..=b';', b'?'..=b'Z', b'a'..=b'z'];
/// PubidChar = ... | [a-zA-Z0-9] | [-()+,./:=?;!*#@$_%]
/// [#..%] | [(../..0..9..;] | [?..@..A..Z]
const PUBID_RANGES_EX_QUOTE: [RangeInclusive<u8>; 4] =
    [b'#'..=b'%', b'('..=b';', b'?'..=b'Z', b'a'..=b'z'];
/// PubidChar = #x20 | #xD | #xA | ... | [!=_]
const PUBID_CHARS: [u8; 6] = [0x20, 0xD, 0xA, b'!', b'_', b'='];

#[derive(Default, Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl TokenTrait for Token {
    type Kind = TokenKind;

    fn kind(&self) -> Self::Kind {
        self.kind
    }
    fn span(&self) -> Span {
        self.span
    }
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn at(kind: TokenKind, at: u32) -> Self {
        Self {
            kind,
            span: Span::splat(at),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
/// Tokens for XML doctype.
/// It's only intended to tokenise `Name`, `ExternalId`, and `GEDecl`.
///
/// For each token kind's doctype, the final line specifies what the token represents.
///
/// doctypedecl = '<!DOCTYPE' S Name (S ExternalID)? S? ('[' intSubset ']' S?)? '>'
///
/// # HTML
///
/// HTML parsers may use this, but only need to parse as follows:
///
/// doctypedecl = '<!DOCTYPE' S [Hh][Tt][Mm][Ll] (S [^>]*) S? >
pub enum TokenKind {
    #[default]
    /// '<!DOCTYPE' S
    DoctypeDefinitionStart,
    /// Name = NameStartChar (NameChar)*
    Name,
    /// ExternalID = 'SYSTEM' S SystemLiteral
    /// 'SYSTEM'
    System,
    /// ExternalID = 'PUBLIC' S PubidLiteral SystemLiteral
    /// 'PUBLIC'
    Public,
    /// SystemLiteral = ('"' [^"]* '"') | ('"' [^']* '"')
    SystemLiteral,
    /// PubidLiteral = '"' PubicChar* '"' | "'" (PubidChar - "'")* "'"
    /// PubidChar = #x20 | #xD | #xA | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]
    PubidLiteral,
    /// '['
    IntSubsetStart,
    /// intSubset = (markupDecl | DeclSep)*
    /// markupDecl = elementDecl | AttlistDecl | EntityDecl | NotationDecl | PI | Comment
    /// DeclSep = PEReference | S
    /// PEReference = '%' Name ';'
    IntSubsetNotEntityDecl,
    /// EntityDecl = GEDecl | PEDecl
    /// GEDecl = '<!ENTITY' S Name S EntityDef S? '>'
    /// PEDecl = '<!ENTITY' S '%' Name S PEDef S? '>'
    /// '<!ENTITY'
    EntityDeclStart,
    /// PEDecl = '<!ENTITY' S '%' Name S PEDef S? '>'
    /// S '%' Name S PEDef S? '>'
    EntityParameterEntityEndInclusive,
    /// GEDecl = '<!ENTITY' S Name S EntityDef S? '>'
    /// Name
    GeneralEntityDeclName,
    /// EntityValue = '"' ([^%&"] | PEReference | Reference)* '"' | "'" ([^%&'] | PEReference | Reference) "'"
    /// Reference = EntityRef | CharRef
    /// EntityRef = '&' Name ';'
    /// CharRef = '&#' [0-9]+ ';' | '&#x' [0-9a-fA-F]+ ';'
    /// PEReference = '%' Name ';'
    /// EntityValue
    GeneralEntityValue,
    /// GEDecl = '<!ENTITY' S Name S EntityDef S? '>'
    /// EntityDef = EntityValue | (ExternalID NDataDecl?)
    /// NDataDecl = S 'NDATA' S Name
    GeneralEntityNotationData,
    /// ']' S?
    IntSubsetEnd,
    /// '>'
    End,
    /// Invalid data that should be treated as an error by the parser
    InvalidData,
    StreamEnd,
}
