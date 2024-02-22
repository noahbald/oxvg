// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

use crate::{cursor::Cursor, diagnostics::SvgParseError, SvgParseErrorMessage};
use std::iter::Peekable;

pub fn is_whitespace(char: &char) -> bool {
    // [3]
    if char == &'\r' {
        println!("Warning, carraige returns should be stripped from document");
    }
    char == &' ' || char == &'\t' || char == &'\r' || char == &'\n'
}

pub fn whitespace(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
    required: bool,
) -> Result<Cursor, Box<SvgParseError>> {
    // [3]
    let mut cursor = cursor;
    let mut has_advanced = false;
    while let Some(&x) = partial.peek() {
        if !is_whitespace(&x) {
            break;
        }

        has_advanced = true;
        cursor.mut_advance();
        if x == '\n' {
            cursor.mut_newline();
        }

        partial.next();
    }

    if required && !has_advanced {
        Err(SvgParseError::new_curse(
            cursor,
            SvgParseErrorMessage::ExpectedWhitespace,
        ))?;
    }

    Ok(cursor)
}

#[test]
fn test_whitespace() {
    let mut file_empty = "".chars().peekable();
    assert_eq!(
        whitespace(&mut file_empty, Cursor::default(), false),
        Ok(Cursor::default()),
        "expect empty string to move cursor by 0"
    );

    let mut file_empty = "".chars().peekable();
    assert_eq!(
        whitespace(&mut file_empty, Cursor::default(), true),
        Err(Box::new(SvgParseError::new_curse(
            Cursor::default(),
            SvgParseErrorMessage::ExpectedWhitespace
        ))),
        "expect required whitespace in empty string to fail"
    );

    let mut file_whitespace = "  Hello, world!".chars().peekable();
    assert_eq!(
        whitespace(&mut file_whitespace, Cursor::default(), true),
        Ok(Cursor::default().advance_by(2)),
        "expect string to move cursor by two column"
    );
    assert_eq!(file_whitespace.next(), Some('H'));

    let mut file_whitespace_end = "  Hello, world!  ".chars().peekable();
    assert_eq!(
        whitespace(&mut file_whitespace_end, Cursor::default(), true),
        Ok(Cursor::default().advance_by(2)),
        "expect string to move cursor by two columns"
    );
    assert_eq!(file_whitespace_end.next(), Some('H'));

    let mut file_newline = "\nHello, world!".chars().peekable();
    assert_eq!(
        whitespace(&mut file_newline, Cursor::default(), true),
        Ok(Cursor::default().newline()),
        "expect string to move cursor to next line"
    );
    assert_eq!(file_newline.next(), Some('H'));

    let mut file_newline_and_space = "  \n Hello, world!".chars().peekable();
    assert_eq!(
        whitespace(&mut file_newline_and_space, Cursor::default(), true),
        Ok(Cursor::default().newline().advance()),
        "expect string to move cursor to next line and 1 column"
    );
    assert_eq!(file_newline_and_space.next(), Some('H'));
}
