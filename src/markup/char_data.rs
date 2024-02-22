// [2.4 Character Data](https://www.w3.org/TR/2006/REC-xml11-20060816/#syntax)

use crate::{cursor::Cursor, diagnostics::SvgParseError};
use std::iter::Peekable;

pub fn char_data(
    partial: &mut Peekable<impl Iterator<Item = char>>,
    cursor: Cursor,
) -> Result<(Cursor, String), Box<SvgParseError>> {
    // [14]
    let mut text: String = "".into();
    while let Some(&char) = partial.peek() {
        match char {
            '&' => break,
            '<' => break,
            _ => {}
        }
        text.push(char);
        cursor.advance();
        partial.next();
    }

    Ok((cursor, text))
}
