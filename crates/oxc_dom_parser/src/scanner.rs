use std::marker::PhantomData;

use crate::{Error, ErrorKind, Span, ast::Str};

pub trait TokeniserTrait<'a, M: MimeCategory> {
    type Token: TokenTrait;

    fn current_token(&self) -> Self::Token;
    fn src_str(&self) -> &'a str;
    fn src(&self) -> &'a [u8];
    fn cursor(&self) -> usize;
    fn set_cursor(&mut self, cursor: usize);
    fn advance(&mut self, by: usize);
    fn force(&mut self, cursor: usize, token: Self::Token);

    fn slice(&self) -> &'a [u8] {
        self.src().get(self.cursor()..).unwrap_or(&[])
    }
    fn current_byte(&self) -> Option<u8> {
        self.src().get(self.cursor()).copied()
    }
    fn current_slice(&self) -> &'a [u8] {
        self.current_token().span().slice(self.src())
    }
    fn current_str(&self) -> Str<'a> {
        self.str(self.current_token().span())
    }
    fn position(&self) -> usize {
        self.cursor()
    }
    fn len(&self) -> usize {
        self.src().len().saturating_sub(self.cursor())
    }
    /// S?
    fn skip_whitespace(&mut self) {
        self.try_next(Self::next_whitespace);
    }
    /// S
    fn next_whitespace(&mut self) -> Result<(), ()> {
        if let Some(position) = M::find_non_whitespace(self.slice()) {
            self.advance(position);
            Ok(())
        } else {
            Err(())
        }
    }
    fn try_next<T, F: Fn(&mut Self) -> Result<T, ()>>(&mut self, f: F) -> Option<T> {
        let start = self.cursor();
        f(self).inspect_err(|_| self.set_cursor(start)).ok()
    }
    fn next_byte(&mut self) -> Option<u8> {
        self.advance(1);
        self.current_byte()
    }
    fn next_until(&mut self, needle: &[u8]) -> Option<()> {
        loop {
            self.next_until_byte(needle[0])?;
            if self.slice().starts_with(needle) {
                self.advance(needle.len());
                return Some(());
            }
            self.advance(1);
        }
    }
    fn next_until_byte(&mut self, byte: u8) -> Option<()> {
        self.advance(crate::simd::find::<true, _>(self.slice(), [byte])?);
        Some(())
    }
    fn next_byte_expect(&mut self, byte: u8) -> Option<()> {
        if self.slice().first().copied() == Some(byte) {
            self.advance(1);
            Some(())
        } else {
            None
        }
    }
    fn next_name_range(&mut self, kind: NameKind) -> Option<()> {
        if !M::is_name_start(self.slice()) {
            // NOTE: Invalid XML
            // `< ` or `<1` or `<name "`
            None
        } else if let Some(end) = M::find_name_end(self.slice(), kind) {
            // `name`
            self.advance(end);
            Some(())
        } else {
            // NOTE: Invalid XML
            //       `TagName` -> `StreamEnd`
            //       `AttrName` -> `StreamEnd`
            // `n<eof>`
            self.end();
            Some(())
        }
    }
    fn end(&mut self) {
        self.set_cursor(self.src_str().len());
    }
    fn at_end(&self) -> bool {
        self.cursor() >= self.src_str().len()
    }
    fn str(&self, range: Span) -> Str<'a> {
        Str::new(&self.src_str()[range.start as usize..range.end as usize])
    }
}

pub trait TokenTrait {
    type Kind;

    fn kind(&self) -> Self::Kind;
    fn span(&self) -> Span;
}

/// Parse following normal XML rules, with HTML exceptions:
///
/// # Elements
/// - Tags are case-insensitive
/// - Void elements have no end tag
/// - Non-void elements cannot self-close
/// - Auto-closing block elements
/// - Mis-nesting is reconstructed
/// - Trailing content after `</html>` or `</body` is moved
/// - `RAWTEXT`, `RCDATA`, `SCRIPT`, and `PLAINTEXT` handling
/// - Whitespace trimming except within `pre`, `textarea`, and `listing`
///
/// # Attributes
/// - Quotes can be omitted
/// - Values can be omitted
/// - Duplicates are ignored
/// - `<` allowed in values
/// - Value normalisation
///
/// # Names
/// - Tag and element names have looser rules
/// - Namespaces are literal instead of URI defined
///
/// # DTD
/// - DTD Not supported
/// - Some character references can omit `;`
/// - Unknown `&<ref>;` is text
///
/// # Self-Repair
/// HTML Document content will never cause an error. Documents are self-healing.
#[cfg(feature = "html")]
pub struct HTML;
/// Parse following normal XML rules
pub struct XML;

pub enum NameKind {
    Element,
    Attribute,
}

pub trait MimeCategory {
    const IS_HTML: bool;
    fn is_name_start(haystack: &[u8]) -> bool;
    fn find_name_end(haystack: &[u8], kind: NameKind) -> Option<usize>;
    fn find_whitespace(haystack: &[u8]) -> Option<usize>;
    fn find_non_whitespace(haystack: &[u8]) -> Option<usize>;
    fn find_whitespace_or_close(haystack: &[u8]) -> Option<usize>;
}
#[cfg(feature = "html")]
impl MimeCategory for HTML {
    const IS_HTML: bool = true;
    fn is_name_start(haystack: &[u8]) -> bool {
        crate::simd::is_html_name_start(haystack)
    }
    fn find_name_end(haystack: &[u8], kind: NameKind) -> Option<usize> {
        match kind {
            NameKind::Element => crate::simd::find_html_tag_name_end::<true>(haystack),
            NameKind::Attribute => crate::simd::find_html_attr_name_end::<true>(haystack),
        }
    }
    fn find_whitespace(haystack: &[u8]) -> Option<usize> {
        crate::simd::find_html_whitespace::<true>(haystack)
    }
    fn find_non_whitespace(haystack: &[u8]) -> Option<usize> {
        crate::simd::find_html_non_whitespace::<true>(haystack)
    }
    fn find_whitespace_or_close(haystack: &[u8]) -> Option<usize> {
        crate::simd::find_html_whitespace_or_close::<true>(haystack)
    }
}
impl MimeCategory for XML {
    const IS_HTML: bool = false;
    fn is_name_start(haystack: &[u8]) -> bool {
        crate::simd::is_xml_name_start(haystack)
    }
    fn find_name_end(haystack: &[u8], _kind: NameKind) -> Option<usize> {
        crate::simd::find_xml_name_end::<true>(haystack)
    }
    fn find_whitespace(haystack: &[u8]) -> Option<usize> {
        crate::simd::find_xml_whitespace::<true>(haystack)
    }
    fn find_non_whitespace(haystack: &[u8]) -> Option<usize> {
        crate::simd::find_xml_non_whitespace::<true>(haystack)
    }
    fn find_whitespace_or_close(haystack: &[u8]) -> Option<usize> {
        crate::simd::find_xml_whitespace_or_close::<true>(haystack)
    }
}

pub struct Tokeniser<'a, M: MimeCategory> {
    pub src_str: &'a str,
    pub src: &'a [u8],
    cursor: usize,
    token: Token,
    is_foreign: u32,
    marker: PhantomData<M>,
}

impl<'a, M: MimeCategory> TokeniserTrait<'a, M> for Tokeniser<'a, M> {
    type Token = Token;

    fn force(&mut self, cursor: usize, token: Self::Token) {
        self.cursor = cursor;
        self.token = token;
    }
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
        self.cursor = cursor;
    }
    fn advance(&mut self, by: usize) {
        self.cursor += by;
    }
}

impl<'a, M: MimeCategory> Iterator for Tokeniser<'a, M> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        Tokeniser::next(self);
        match self.current_token().kind() {
            TokenKind::StreamEnd => None,
            _ => Some(self.token),
        }
    }
}

impl<'a, M: MimeCategory> Tokeniser<'a, M> {
    pub fn new(source: &'a str) -> Self {
        let src = source.as_bytes();
        Self {
            src_str: source,
            src,
            cursor: 0,
            token: Token::default(),
            is_foreign: if M::IS_HTML { 0 } else { 1 },
            marker: PhantomData,
        }
    }

    pub fn next(&mut self) {
        self.token = if self.at_end() {
            Token::at(TokenKind::StreamEnd, self.cursor as u32)
        } else {
            match self.token.kind {
                TokenKind::TagOpenStart | TokenKind::TagCloseStart => self.next_tag_name(),
                TokenKind::TagName
                | TokenKind::ValueSingleQuote
                | TokenKind::ValueDoubleQuote
                | TokenKind::ValueUnquoted => self.next_attr_name(),
                TokenKind::AttrName => self.next_attr_value(),
                TokenKind::CommentStart => self.next_comment(),
                TokenKind::CDataStart => self.next_c_data(),
                TokenKind::PIStart => self.next_pi_target(),
                TokenKind::PITarget => self.next_pi_text(),
                TokenKind::DoctypeStart => self.next_doctype_name(),
                TokenKind::DoctypeName => self.next_doctype_content(),
                TokenKind::DocumentStart
                | TokenKind::Text
                | TokenKind::TextHTMLFallback
                | TokenKind::TagEnd
                | TokenKind::TagSelfEnd
                | TokenKind::CommentTextEndInclusive
                | TokenKind::CDataTextEndInclusive
                | TokenKind::PIContentEndInclusive
                | TokenKind::DoctypeContentEndInclusive => self.next_start(),
                TokenKind::StreamEnd => {
                    debug_assert!(self.at_end());
                    Token::at(TokenKind::StreamEnd, self.cursor as u32)
                }
            }
        };
    }

    fn next_start(&mut self) -> Token {
        let start = self.cursor;
        match self.current_byte() {
            Some(b'<') => match self.next_byte() {
                // `<!`
                Some(b'!') => {
                    self.advance(1);
                    self.next_bang(start)
                }
                // `<?`
                Some(b'?') if !M::IS_HTML => {
                    self.advance(1);
                    Token::new(TokenKind::PIStart, Span::new_usize(start, self.cursor))
                }
                // </
                Some(b'/') => {
                    self.advance(1);
                    Token::new(
                        TokenKind::TagCloseStart,
                        Span::new_usize(start, self.cursor),
                    )
                }
                _ => Token::new(TokenKind::TagOpenStart, Span::new_usize(start, self.cursor)),
            },
            _ => self.next_text(),
        }
    }

    fn next_text(&mut self) -> Token {
        let start = self.cursor;
        self.cursor +=
            crate::simd::find::<true, _>(self.slice(), [b'<']).unwrap_or(self.slice().len());
        Token::new(TokenKind::Text, Span::new_usize(start, self.cursor))
    }

    fn next_bang(&mut self, start: usize) -> Token {
        if self.slice().starts_with(b"--") {
            // `<!--`
            self.cursor += 2;
            Token::new(TokenKind::CommentStart, Span::new_usize(start, self.cursor))
        } else if !M::IS_HTML && self.slice().starts_with(b"[CDATA[") {
            // `<![CDATA[`
            self.cursor += 7;
            Token::new(TokenKind::CDataStart, Span::new_usize(start, self.cursor))
        } else if self.start_with_doctype() {
            // `<!DOCTYPE`
            self.advance(7);
            Token::new(TokenKind::DoctypeStart, Span::new_usize(start, self.cursor))
        } else {
            // NOTE: Invalid XML
            Token::new(
                TokenKind::TextHTMLFallback,
                Span::new_usize(start, self.cursor),
            )
        }
    }

    fn next_tag_name(&mut self) -> Token {
        let start = self.cursor;
        let kind = if self.next_name_range(NameKind::Element).is_some() {
            TokenKind::TagName
        } else {
            TokenKind::TextHTMLFallback
        };
        Token::new(kind, Span::new_usize(start, self.cursor))
    }

    fn next_attr_name(&mut self) -> Token {
        loop {
            self.skip_whitespace();
            let start = self.cursor;
            return match self.current_byte() {
                Some(b'>') => {
                    // `<tag attr="value">`
                    self.advance(1);
                    Token::new(TokenKind::TagEnd, Span::new_usize(start, self.cursor))
                }
                Some(b'/') => {
                    // `<tag attr="value"/>`
                    if self.slice().starts_with(b"/>") {
                        self.advance(2);
                        return Token::new(
                            TokenKind::TagSelfEnd,
                            Span::new_usize(start, self.cursor),
                        );
                    }
                    // `<tag attr"value"/...`
                    if M::IS_HTML {
                        // Garbage `/` is ignored
                        self.advance(1);
                        continue;
                    } else {
                        // NOTE: Invalid XML
                        Token::at(TokenKind::TextHTMLFallback, start as u32)
                    }
                }
                None => Token::at(TokenKind::StreamEnd, start as u32),
                _ => {
                    if self.next_name_range(NameKind::Attribute).is_some() {
                        Token::new(TokenKind::AttrName, Span::new_usize(start, self.cursor))
                    } else {
                        Token::new(
                            TokenKind::TextHTMLFallback,
                            Span::new_usize(start, self.cursor),
                        )
                    }
                }
            };
        }
    }

    fn next_attr_value(&mut self) -> Token {
        self.skip_whitespace();
        let start = self.cursor;
        match self.current_byte() {
            Some(b'>') => {
                // NOTE: Invalid XML
                // `<tag attr>`
                self.advance(1);
                return Token::at(TokenKind::TagEnd, self.cursor as u32);
            }
            Some(b'/') if self.slice().starts_with(b"/>") => {
                // NOTE: Invalid XML
                // `<tag attr/>`
                self.advance(2);
                return Token::new(TokenKind::TagSelfEnd, Span::new_usize(start, self.cursor));
            }
            Some(b'=') => {}
            None => return Token::at(TokenKind::StreamEnd, start as u32),
            // NOTE: Invalid XML
            // `<tag attr attr`
            _ => return self.next_attr_name(),
        }

        self.advance(1);
        self.skip_whitespace();
        let value_start = self.cursor;
        let Some(quote) = self.current_byte() else {
            return Token::at(TokenKind::StreamEnd, value_start as u32);
        };
        match quote {
            // `<tag attr="value"` or `<tag attr='value'`
            b'"' | b'\'' => {
                let kind = if quote == b'"' {
                    TokenKind::ValueDoubleQuote
                } else {
                    TokenKind::ValueSingleQuote
                };
                if let Some(i) = crate::simd::find::<true, _>(&self.src[value_start + 1..], [quote])
                {
                    self.advance(i + 2);
                    Token::new(kind, Span::new_usize(value_start, self.cursor))
                } else {
                    self.end();
                    Token::new(
                        TokenKind::TextHTMLFallback,
                        Span::new_usize(value_start, self.cursor),
                    )
                }
            }
            // NOTE: Invalid XML
            // `<tag attr=value`
            // FIXME: Join simd instead of using `min`
            _ => {
                let i = M::find_whitespace_or_close(self.slice()).unwrap_or(self.slice().len());
                self.advance(i);
                Token::new(
                    TokenKind::ValueUnquoted,
                    Span::new_usize(value_start, self.cursor as usize),
                )
            }
        }
    }

    fn next_comment(&mut self) -> Token {
        let start = self.cursor;
        if M::IS_HTML {
            if self.slice().starts_with(b">") {
                // `<!-->`
                self.advance(1);
                return Token::new(
                    TokenKind::CommentTextEndInclusive,
                    Span::new_usize(start, self.cursor),
                );
            } else if self.slice().starts_with(b"->") {
                // `<!--->`
                self.advance(2);
                return Token::new(
                    TokenKind::CommentTextEndInclusive,
                    Span::new_usize(start, self.cursor),
                );
            }
        }
        while !self.at_end() {
            if let Some(pos) = crate::simd::find::<true, _>(self.slice(), [b'-']) {
                self.advance(pos);
                if self.slice().starts_with(b"-->") {
                    // `<!-- <text> -->`
                    self.advance(3);
                    return Token::new(
                        TokenKind::CommentTextEndInclusive,
                        Span::new_usize(start, self.cursor),
                    );
                } else if M::IS_HTML && self.slice().starts_with(b"--!>") {
                    // `<!-- <text> --!>`
                    self.advance(4);
                    return Token::new(
                        TokenKind::CommentTextEndInclusive,
                        Span::new_usize(start, self.cursor),
                    );
                } else {
                    self.advance(1);
                    continue;
                }
            } else {
                break;
            };
        }
        self.end();
        // NOTE: Invalid XML
        // `<!-- <eof>`
        let kind = if M::IS_HTML {
            TokenKind::CommentTextEndInclusive
        } else {
            TokenKind::TextHTMLFallback
        };
        Token::new(kind, Span::new_usize(start, self.cursor))
    }

    fn next_c_data(&mut self) -> Token {
        debug_assert!(!M::IS_HTML);
        let start = self.cursor;
        if self.next_until(b"]]>").is_some() {
            // `<![CDATA[<text>]]>`
            Token::new(
                TokenKind::CDataTextEndInclusive,
                Span::new_usize(start, self.cursor),
            )
        } else {
            // NOTE: Invalid XML
            // `<![CDATA[<eof>`
            Token::new(
                TokenKind::TextHTMLFallback,
                Span::new_usize(start, self.cursor),
            )
        }
    }

    fn next_pi_target(&mut self) -> Token {
        debug_assert!(!M::IS_HTML);
        self.skip_whitespace();
        let start = self.cursor;
        if self.next_name_range(NameKind::Element).is_some() {
            // `<? target`
            Token::new(TokenKind::PITarget, Span::new_usize(start, self.cursor))
        } else {
            // NOTE: Invalid XML
            Token::new(
                TokenKind::TextHTMLFallback,
                Span::new_usize(start, self.cursor),
            )
        }
    }

    fn next_pi_text(&mut self) -> Token {
        debug_assert!(!M::IS_HTML);
        self.skip_whitespace();
        let start = self.cursor;
        if self.next_until(b"?>").is_some() {
            Token::new(
                TokenKind::PIContentEndInclusive,
                Span::new_usize(start, self.cursor),
            )
        } else {
            // NOTE Invalid XML
            Token::new(
                TokenKind::TextHTMLFallback,
                Span::new_usize(start, self.cursor),
            )
        }
    }

    fn next_doctype_name(&mut self) -> Token {
        let before = self.cursor;
        self.skip_whitespace();
        let start = self.cursor;
        if M::IS_HTML {
            let len = M::find_whitespace_or_close(self.slice()).unwrap_or(self.slice().len());
            self.advance(len);
            Token::new(TokenKind::DoctypeName, Span::new_usize(start, self.cursor))
        } else if self.cursor != before && self.next_name_range(NameKind::Element).is_some() {
            Token::new(TokenKind::DoctypeName, Span::new_usize(start, self.cursor))
        } else {
            // NOTE: Inavlid XML
            Token::new(TokenKind::TextHTMLFallback, Span::splat(start as u32))
        }
    }

    fn next_doctype_content(&mut self) -> Token {
        self.skip_whitespace();
        let start = self.cursor;
        let end = if M::IS_HTML {
            // The first `>` always ends it, even inside quoted identifiers
            crate::simd::find::<true, _>(self.slice(), [b'>']).map(|i| i + 1)
        } else {
            self.find_xml_doctype_end()
        };
        match end {
            Some(len) => {
                self.advance(len);
                Token::new(
                    TokenKind::DoctypeContentEndInclusive,
                    Span::new_usize(start, self.cursor),
                )
            }
            None => {
                self.end();
                // `<!DOCTYPE html <eof>`: HTML closes the doctype implicitly
                let kind = if M::IS_HTML {
                    TokenKind::DoctypeContentEndInclusive
                } else {
                    TokenKind::TextHTMLFallback
                };
                Token::new(kind, Span::new_usize(start, self.cursor))
            }
        }
    }

    fn start_with_doctype(&self) -> bool {
        let start = self.slice();
        if M::IS_HTML {
            start.len() >= 7 && start[..7].eq_ignore_ascii_case(b"DOCTYPE")
        } else {
            start.starts_with(b"DOCTYPE")
        }
    }

    #[cold] // called once per document
    fn find_xml_doctype_end(&self) -> Option<usize> {
        let s = self.slice();
        let mut i = 0;
        let mut in_subset = false;
        while i < s.len() {
            match s[i] {
                quote @ (b'"' | b'\'') => {
                    i += s[i + 1..].iter().position(|&b| b == quote)? + 2;
                }
                b'[' if !in_subset => {
                    in_subset = true;
                    i += 1;
                }
                b']' if in_subset => {
                    in_subset = false;
                    i += 1;
                }
                b'<' if in_subset && s[i..].starts_with(b"<!--") => {
                    i += 4 + find_subslice(&s[i + 4..], b"-->")? + 3;
                }
                b'<' if in_subset && s[i..].starts_with(b"<?") => {
                    i += 2 + find_subslice(&s[i + 2..], b"?>")? + 2;
                }
                b'>' if !in_subset => return Some(i + 1),
                _ => i += 1,
            }
        }
        None
    }
}

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
    fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn at(kind: TokenKind, at: u32) -> Self {
        Self {
            kind,
            span: Span::splat(at),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum TokenKind {
    #[default]
    /// Start of file
    DocumentStart,
    /// `<`
    TagOpenStart,
    /// `</`
    TagCloseStart,
    /// Name of a tag.
    ///
    /// Guaranteed to be have valid name start/chars for given XML/HTML.
    TagName,
    /// `>`
    TagEnd,
    /// `/>`
    TagSelfEnd,
    /// `<!--`
    CommentStart,
    /// `<comment text>-->`
    CommentTextEndInclusive,
    /// `<![CDATA[`
    ///
    /// Treated as text in non-foreign HTML.
    CDataStart,
    /// `<c-data text>]]>`
    ///
    /// Treated as text in non-foreign HTML.
    CDataTextEndInclusive,
    /// `<?`
    PIStart,
    /// Text between `<?` and `<content>?>`
    PITarget,
    /// `<content>?>`
    PIContentEndInclusive,
    /// `<!DOCTYPE`
    DoctypeStart,
    /// Name of doctype
    DoctypeName,
    /// `<content>>`
    ///
    /// HTML may cut off `>` with early EOF.
    DoctypeContentEndInclusive,
    /// Text content
    Text,
    /// Text content.
    ///
    /// Invalid in XML.
    TextHTMLFallback,
    /// Name of an attribute
    /// `attr`
    AttrName,
    /// Value of an attribute.
    /// Guaranteed to not contain unescaped `'`.
    /// `attr='value'`
    ValueSingleQuote,
    /// Value of an attribute.
    /// Guaranteed to not contain unescaped `"`.
    /// `attr="value"`
    ValueDoubleQuote,
    /// Unquoted value of an attribute.
    /// `attr=value`
    ///
    /// Invalid in XML.
    ValueUnquoted,
    /// End of file.
    ///
    /// Invalid XML node stack is incomplete.
    ///
    /// Always valid HTML; should drain node stack.
    StreamEnd,
}

impl Token {
    pub fn check_xml_transition(self, next: Self) -> Result<(), Error> {
        match next.kind {
            TokenKind::DocumentStart => unreachable!(),
            TokenKind::TextHTMLFallback => Err(Error::new(ErrorKind::UnrecognisedData, next.span)),
            TokenKind::StreamEnd => match self.kind {
                TokenKind::TagName | TokenKind::AttrName => {
                    Err(Error::new(ErrorKind::UnexpectedEndOfFile, next.span))
                }
                TokenKind::CommentStart => {
                    Err(Error::new(ErrorKind::UnexpectedEndOfFile, next.span))
                }
                _ => Ok(()),
            },
            TokenKind::TagEnd | TokenKind::TagSelfEnd => match self.kind {
                TokenKind::AttrName => Err(Error::new(ErrorKind::UnexpectedCloseTag, next.span)),
                _ => Ok(()),
            },
            TokenKind::ValueUnquoted => Err(Error::new(
                ErrorKind::DisallowedUnquotedAttributeValue,
                next.span,
            )),
            TokenKind::AttrName => match self.kind {
                TokenKind::AttrName => Err(Error::new(
                    ErrorKind::DisallowedMissingAttributeValue,
                    self.span,
                )),
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod test {
    use crate::scanner::{TokenKind, Tokeniser, XML};

    #[test]
    fn xml_well_formed() {
        let mut tokeniser = Tokeniser::<XML>::new(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- comment -->
<element>
  <child attribute="value" />
  <![CDATA[ cdata ]]>
</element>"#,
        );
        assert_eq!(tokeniser.current_token_kind(), TokenKind::DocumentStart);
        assert_eq!(tokeniser.current_str().as_str(), "");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::PIStart);
        assert_eq!(tokeniser.current_str().as_str(), "<?");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::PITarget);
        assert_eq!(tokeniser.current_str().as_str(), "xml");

        tokeniser.next();
        assert_eq!(
            tokeniser.current_token_kind(),
            TokenKind::PIContentEndInclusive
        );
        assert_eq!(
            tokeniser.current_str().as_str(),
            r#"version="1.0" encoding="UTF-8"?>"#
        );

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::Text);
        assert_eq!(tokeniser.current_str().as_str(), "\n");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::CommentStart);
        assert_eq!(tokeniser.current_str().as_str(), "<!--");

        tokeniser.next();
        assert_eq!(
            tokeniser.current_token_kind(),
            TokenKind::CommentTextEndInclusive
        );
        assert_eq!(tokeniser.current_str().as_str(), " comment -->");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::Text);
        assert_eq!(tokeniser.current_str().as_str(), "\n");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagOpenStart);
        assert_eq!(tokeniser.current_str().as_str(), "<");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagName);
        assert_eq!(tokeniser.current_str().as_str(), "element");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagEnd);
        assert_eq!(tokeniser.current_str().as_str(), ">");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::Text);
        assert_eq!(tokeniser.current_str().as_str(), "\n  ");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagOpenStart);
        assert_eq!(tokeniser.current_str().as_str(), "<");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagName);
        assert_eq!(tokeniser.current_str().as_str(), "child");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::AttrName);
        assert_eq!(tokeniser.current_str().as_str(), "attribute");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::ValueDoubleQuote);
        assert_eq!(tokeniser.current_str().as_str(), r#""value""#);

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagSelfEnd);
        assert_eq!(tokeniser.current_str().as_str(), "/>");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::Text);
        assert_eq!(tokeniser.current_str().as_str(), "\n  ");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::CDataStart);
        assert_eq!(tokeniser.current_str().as_str(), "<![CDATA[");

        tokeniser.next();
        assert_eq!(
            tokeniser.current_token_kind(),
            TokenKind::CDataTextEndInclusive
        );
        assert_eq!(tokeniser.current_str().as_str(), " cdata ]]>");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::Text);
        assert_eq!(tokeniser.current_str().as_str(), "\n");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagCloseStart);
        assert_eq!(tokeniser.current_str().as_str(), "</");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagName);
        assert_eq!(tokeniser.current_str().as_str(), "element");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::TagEnd);
        assert_eq!(tokeniser.current_str().as_str(), ">");

        tokeniser.next();
        assert_eq!(tokeniser.current_token_kind(), TokenKind::StreamEnd);
        assert_eq!(tokeniser.current_str().as_str(), "");
    }
}
