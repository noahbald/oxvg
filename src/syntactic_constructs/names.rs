// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

use crate::{cursor::Cursor, diagnostics::SvgParseError, SvgParseErrorMessage};
use std::iter::Peekable;

pub fn is_name_start_char(char: &char) -> bool {
    // [4]
    if match char {
        '_' => true,
        ':' => true,
        c if c.is_uppercase() => true,
        c if c.is_lowercase() => true,
        _ => false,
    } {
        return true;
    }

    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = utf16[0] as u32 | (utf16[1] as u32) << 16;
    (0xC0..=0xD6).contains(&utf16)
        || (0xD8..=0xF6).contains(&utf16)
        || (0xF8..=0x2FF).contains(&utf16)
        || (0x370..=0x37D).contains(&utf16)
        || (0x37F..=0x1FFF).contains(&utf16)
        || (0x200C..=0x200D).contains(&utf16)
        || (0x2070..=0x218F).contains(&utf16)
        || (0x2C00..=0x2FEF).contains(&utf16)
        || (0x3001..=0xD7FF).contains(&utf16)
        || (0xF900..=0xFDCF).contains(&utf16)
        || (0xFDF0..=0xFFFD).contains(&utf16)
        || (0x10000..=0xEFFFF).contains(&utf16)
}

pub fn is_name_char(char: &char) -> bool {
    // [4a]
    if match char {
        c if is_name_start_char(c) => true,
        '-' => true,
        '.' => true,
        c if c.is_numeric() => true,
        _ => false,
    } {
        return true;
    }

    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = utf16[0];
    utf16 == 0xB7 || (0x0300..0x036F).contains(&utf16) || (0x203F..0x2040).contains(&utf16)
}

enum Construct {
    Name,
    NameToken,
}

static NAME_EXPECTED: &str = "valid starting name character";

pub fn name(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, String), Box<SvgParseError>> {
    // [5]
    let mut cursor = cursor;
    let mut text = "".to_string();

    while let Some(next_char) = partial.peek() {
        if text.is_empty() && !is_name_start_char(next_char) {
            Err(SvgParseError::new_curse(
                cursor.advance(),
                SvgParseErrorMessage::UnexpectedChar(*next_char, NAME_EXPECTED.into()),
            ))?
        }
        if !is_name_char(next_char) {
            break;
        }

        cursor.mut_advance();
        text.push(partial.next().unwrap());
    }

    if text.is_empty() {
        Err(SvgParseError::new_curse(
            cursor.advance(),
            SvgParseErrorMessage::ExpectedWord,
        ))?
    }

    Ok((cursor, text))
}

#[test]
fn test_name() {
    let mut word = "Hello, world!".chars().peekable();
    assert_eq!(
        name(&mut word, Cursor::default()),
        Ok((Cursor::default().advance_by(5), "Hello".into())),
    );
    assert_eq!(word.next(), Some(','));

    let mut no_word = "".chars().peekable();
    assert_eq!(
        name(&mut no_word, Cursor::default()),
        Err(Box::new(SvgParseError::new_curse(
            Cursor::default(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        )))
    );

    let mut leading_whitespace = " Hello, world!".chars().peekable();
    assert_eq!(
        name(&mut leading_whitespace, Cursor::default()),
        Err(Box::new(SvgParseError::new_curse(
            Cursor::default().newline(),
            SvgParseErrorMessage::UnexpectedChar(' ', NAME_EXPECTED.into())
        )))
    );
    assert_eq!(leading_whitespace.next(), Some(' '));

    let mut includes_permitted_name_chars = ":_-.Aa ".chars().peekable();
    assert_eq!(
        name(&mut includes_permitted_name_chars, Cursor::default()),
        Ok((Cursor::default().advance_by(6), ":_-.Aa".into()))
    );
    assert_eq!(includes_permitted_name_chars.next(), Some(' '));
}

pub fn names(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, Vec<String>), Box<SvgParseError>> {
    // [6]
    collect_construct(partial, cursor, Construct::Name)
}

pub fn nm_token(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, String), Box<SvgParseError>> {
    let mut nm_token_output = "".to_string();
    let mut cursor = cursor;
    while let Some(char) = partial.peek() {
        if !is_name_char(char) {
            break;
        }

        nm_token_output.push(*char);
        cursor.mut_advance();
        partial.next();
    }

    if nm_token_output.is_empty() {
        Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::ExpectedWord,
        ))?
    }

    Ok((cursor, nm_token_output))
}

pub fn nm_tokens(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, Vec<String>), Box<SvgParseError>> {
    // [8]
    collect_construct(partial, cursor, Construct::NameToken)
}

fn collect_construct(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    construct: Construct,
) -> Result<(Cursor, Vec<String>), Box<SvgParseError>> {
    let construct = match construct {
        Construct::Name => name,
        Construct::NameToken => nm_token,
    };
    let (mut cursor, current_name) = construct(partial, cursor)?;
    let mut names_output = vec![current_name];

    loop {
        if partial.peek() != Some(&' ') {
            break;
        }
        partial.next();

        match partial.peek() {
            Some(c) if is_name_char(c) => {}
            _ => break,
        }

        let (c, current_name) = construct(partial, cursor)?;
        cursor = c;
        names_output.push(current_name);
    }

    Ok((cursor, names_output))
}
