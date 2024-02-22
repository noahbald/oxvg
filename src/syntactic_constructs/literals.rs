// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

use std::iter::Peekable;

use crate::{
    cursor::{Cursor, Span},
    diagnostics::SvgParseError,
    references::reference,
    SvgParseErrorMessage,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LiteralValue(Span, String);

impl LiteralValue {
    pub fn new(cursor: Cursor, value: String) -> Self {
        Self(cursor.as_span(value.len()), value)
    }
}

pub enum Literal {
    EntityValue,
    AttValue,
    SystemLiteral,
    PubidLiteral,
}

pub fn literal(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    expected: Literal,
) -> Result<(Cursor, LiteralValue), Box<SvgParseError>> {
    let mut text: String = "".into();
    let cursor_start = cursor;
    let mut cursor = cursor;

    cursor.mut_advance();
    let quote_style = match partial.next() {
        Some('\'') => '\'',
        Some('"') => '"',
        Some(c) => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedChar(c, "An opening `'` or `\"`".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };
    text.push(quote_style);

    while let Some(char) = partial.next() {
        cursor.mut_advance();
        text.push(char);
        if char == quote_style {
            break;
        }

        match expected {
            Literal::EntityValue if char == '&' || char == '%' => {
                // [9]
                let (c, ref_item) = reference(partial, cursor)?;
                cursor = c;
                text.push_str(&ref_item.unwrap());
            }
            Literal::EntityValue => Err(SvgParseError::new_curse(
                cursor,
                SvgParseErrorMessage::UnexpectedChar(
                    char,
                    "Start of reference (ie. `%__;` or `&__;`)".into(),
                ),
            ))?,
            Literal::AttValue if char == '&' => {
                // [10]
                let (c, ref_item) = reference(partial, cursor)?;
                cursor = c;
                text.push_str(&ref_item.unwrap());
            }
            Literal::AttValue => {}
            Literal::SystemLiteral => {
                // [11]
            }
            Literal::PubidLiteral if is_pubid_char(&char) => {
                // [12]
            }
            Literal::PubidLiteral => Err(SvgParseError::new_curse(
                cursor,
                SvgParseErrorMessage::UnexpectedChar(
                    char,
                    "valid public identifier literal character".into(),
                ),
            ))?,
        }
    }

    Ok((cursor, LiteralValue::new(cursor_start, text)))
}

fn is_pubid_char(char: &char) -> bool {
    // [13]
    char == &' '
        || char == &'\r'
        || char == &'\n'
        || char.is_ascii_lowercase()
        || char.is_ascii_uppercase()
        || char.is_ascii_digit()
        || char == &'!'
        || ('#'..='%').contains(char)
        || ('\''..='/').contains(char)
        || char == &':'
        || char == &';'
        || char == &'='
        || char == &'?'
        || char == &'@'
        || char == &'_'
}
