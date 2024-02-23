// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

use crate::{cursor::Cursor, diagnostics::SvgParseError, SvgParseErrorMessage};
use std::iter::Peekable;

static NAME_EXPECTED: &str = "valid starting name character";

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Name(String);

impl Name {
    pub fn new(
        partial: &mut Peekable<impl Iterator<Item = char>>,
        cursor: Cursor,
    ) -> Result<(Cursor, Self), Box<SvgParseError>> {
        // [5]
        let mut cursor = cursor;
        let mut text = "".to_string();

        while let Some(next_char) = partial.peek() {
            if text.is_empty() && !Self::is_name_start_char(next_char) {
                Err(SvgParseError::new_curse(
                    cursor.advance(),
                    SvgParseErrorMessage::UnexpectedChar(*next_char, NAME_EXPECTED.into()),
                ))?
            }
            if !Self::is_name_char(next_char) {
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

        Ok((cursor, Self(text)))
    }

    pub fn is_name_char(char: &char) -> bool {
        // [4a]
        if match char {
            c if Self::is_name_start_char(c) => true,
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

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn to_lowercase(&self) -> String {
        self.0.to_lowercase()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl Into<String> for Name {
    fn into(self) -> String {
        self.0
    }
}

#[test]
fn test_name() {
    let mut word = "Hello, world!".chars().peekable();
    assert_eq!(
        Name::new(&mut word, Cursor::default()),
        Ok((Cursor::default().advance_by(5), "Hello".into())),
    );
    assert_eq!(word.next(), Some(','));

    let mut no_word = "".chars().peekable();
    assert_eq!(
        Name::new(&mut no_word, Cursor::default()),
        Err(Box::new(SvgParseError::new_curse(
            Cursor::default(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        )))
    );

    let mut leading_whitespace = " Hello, world!".chars().peekable();
    assert_eq!(
        Name::new(&mut leading_whitespace, Cursor::default()),
        Err(Box::new(SvgParseError::new_curse(
            Cursor::default().newline(),
            SvgParseErrorMessage::UnexpectedChar(' ', NAME_EXPECTED.into())
        )))
    );
    assert_eq!(leading_whitespace.next(), Some(' '));

    let mut includes_permitted_name_chars = ":_-.Aa ".chars().peekable();
    assert_eq!(
        Name::new(&mut includes_permitted_name_chars, Cursor::default()),
        Ok((Cursor::default().advance_by(6), ":_-.Aa".into()))
    );
    assert_eq!(includes_permitted_name_chars.next(), Some(' '));
}
