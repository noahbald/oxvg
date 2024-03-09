// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

use crate::{cursor::Cursor, diagnostics::SVGError, file_reader::FileReader, SVGErrorLabel};
use std::iter::Peekable;

pub fn is_whitespace(char: &char) -> bool {
    // [3]
    if char == &'\r' {
        // println!("Warning, carraige returns should be stripped from document");
    }
    char == &' ' || char == &'\t' || char == &'\r' || char == &'\n'
}

pub fn whitespace(partial: &mut FileReader, required: bool) -> Result<(), Box<SVGError>> {
    // [3]
    let mut has_advanced = false;
    while let Some(&x) = partial.peek() {
        if !is_whitespace(&x) {
            break;
        }

        has_advanced = true;
        partial.next();
    }

    if required && !has_advanced {
        Err(SVGError::new_curse(
            partial.get_cursor(),
            SVGErrorLabel::ExpectedWhitespace,
        ))?;
    }

    Ok(())
}

#[test]
fn test_whitespace() {
    let mut file_empty = FileReader::new("");
    assert_eq!(
        whitespace(&mut file_empty, false),
        Ok(()),
        "expect empty string to move cursor by 0"
    );

    let mut file_empty = FileReader::new("");
    assert_eq!(
        whitespace(&mut file_empty, true),
        Err(Box::new(SVGError::new_curse(
            Cursor::default(),
            SVGErrorLabel::ExpectedWhitespace
        ))),
        "expect required whitespace in empty string to fail"
    );

    let mut file_whitespace = FileReader::new("  Hello, world!");
    assert_eq!(
        whitespace(&mut file_whitespace, true),
        Ok(()),
        "expect string to move cursor by two column"
    );
    assert_eq!(file_whitespace.next(), Some('H'));

    let mut file_whitespace_end = FileReader::new("  Hello, world!  ");
    assert_eq!(
        whitespace(&mut file_whitespace_end, true),
        Ok(()),
        "expect string to move cursor by two columns"
    );
    assert_eq!(file_whitespace_end.next(), Some('H'));

    let mut file_newline = FileReader::new("\nHello, world!");
    assert_eq!(
        whitespace(&mut file_newline, true),
        Ok(()),
        "expect string to move cursor to next line"
    );
    assert_eq!(file_newline.next(), Some('H'));

    let mut file_newline_and_space = FileReader::new("  \n Hello, world!");
    assert_eq!(
        whitespace(&mut file_newline_and_space, true),
        Ok(()),
        "expect string to move cursor to next line and 1 column"
    );
    assert_eq!(file_newline_and_space.next(), Some('H'));
}
