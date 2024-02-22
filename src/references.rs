// [4.1 Character and Entity References](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-references)

use crate::characters::{char, is_char};
use crate::syntactic_constructs::name;
use crate::{cursor::Cursor, diagnostics::SvgParseError, SvgParseErrorMessage};
use std::iter::Peekable;

#[derive(PartialEq, Debug)]
pub enum Reference {
    Char(String),
    Entity(String),
    ParameterEntity(String),
}

impl Reference {
    pub fn unwrap(&self) -> String {
        match self {
            Self::Char(s) | Self::Entity(s) | Self::ParameterEntity(s) => s.into(),
        }
    }
}

pub fn reference(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, Reference), Box<SvgParseError>> {
    let mut text: String = "".into();
    let cursor = cursor.advance();
    let cursor_start = cursor;
    let is_pe_ref = match partial.next() {
        Some('&') => {
            // [67]
            text.push('&');
            false
        }
        Some('%') => {
            // [69]
            text.push('%');
            true
        }
        Some(c) => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedChar(c, "& or %".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };

    match partial.peek() {
        Some('#') if !is_pe_ref => {
            // [66]
            text.push('#')
        }
        Some(&c) => {
            let cursor = char(partial, cursor, Some(c))?;
            let (cursor, ref_name) = name(partial, cursor)?;
            text.push_str(&ref_name);
            let cursor = char(partial, cursor, Some(';'))?;
            text.push(';');
            return Ok((
                cursor,
                match is_pe_ref {
                    // [69]
                    true => Reference::ParameterEntity(text),
                    // [68]
                    false => Reference::Entity(text),
                },
            ));
        }
        None => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };
    let cursor = cursor.advance();
    partial.next();

    let cursor = cursor.advance();
    let is_hex = match partial.next() {
        Some('x') => {
            text.push('x');
            true
        }
        Some(c) if c.is_numeric() => {
            text.push(c);
            false
        }
        Some(c) => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedChar(c, "x or number".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };

    loop {
        let cursor = cursor.advance();
        match partial.next() {
            Some(';') => {
                text.push(';');
                break;
            }
            Some(c) if c.is_numeric() => text.push(c),
            Some(c) if is_hex && ('a'..='f').contains(&c) || ('A'..='F').contains(&c) => {
                text.push(c)
            }
            Some(c) => Err(SvgParseError::new_curse(
                cursor,
                SvgParseErrorMessage::UnexpectedChar(c, "number or hex".into()),
            ))?,
            None => Err(SvgParseError::new_curse(
                cursor,
                SvgParseErrorMessage::UnexpectedEndOfFile,
            ))?,
        };
    }

    let char = match u8::from_str_radix(&text[1..text.len() - 1], 16) {
        Ok(u) => char::from(u),
        Err(_) => Err(SvgParseError::new_span(
            cursor_start.as_span(text.len()),
            SvgParseErrorMessage::IllegalCharRef(text.clone()),
        ))?,
    };
    if !is_char(&char) {
        Err(SvgParseError::new_span(
            cursor_start.as_span(text.len()),
            SvgParseErrorMessage::IllegalCharRef(text.clone()),
        ))?;
    };
    Ok((cursor, Reference::Char(text)))
}
