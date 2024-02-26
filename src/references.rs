// [4.1 Character and Entity References](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-references)

use crate::characters::{char, is_char};
use crate::file_reader::FileReader;
use crate::syntactic_constructs::Name;
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

pub fn reference(file_reader: &mut FileReader) -> Result<Reference, Box<SvgParseError>> {
    let mut text: String = "".into();
    let cursor_start = file_reader.get_cursor();
    let is_pe_ref = match file_reader.next() {
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
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedChar(c, "& or %".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };

    match file_reader.peek() {
        Some('#') if !is_pe_ref => {
            // [66]
            text.push('#')
        }
        Some(&c) => {
            char(file_reader, Some(c))?;
            let ref_name = Name::new(file_reader)?;
            text.push_str(ref_name.as_str());
            char(file_reader, Some(';'))?;
            text.push(';');
            return Ok(match is_pe_ref {
                // [69]
                true => Reference::ParameterEntity(text),
                // [68]
                false => Reference::Entity(text),
            });
        }
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };
    file_reader.next();

    let is_hex = match file_reader.next() {
        Some('x') => {
            text.push('x');
            true
        }
        Some(c) if c.is_numeric() => {
            text.push(c);
            false
        }
        Some(c) => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedChar(c, "x or number".into()),
        ))?,
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };

    loop {
        match file_reader.next() {
            Some(';') => {
                text.push(';');
                break;
            }
            Some(c) if c.is_numeric() => text.push(c),
            Some(c) if is_hex && ('a'..='f').contains(&c) || ('A'..='F').contains(&c) => {
                text.push(c)
            }
            Some(c) => Err(SvgParseError::new_curse(
                file_reader.get_cursor(),
                SvgParseErrorMessage::UnexpectedChar(c, "number or hex".into()),
            ))?,
            None => Err(SvgParseError::new_curse(
                file_reader.get_cursor(),
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
    Ok(Reference::Char(text))
}
