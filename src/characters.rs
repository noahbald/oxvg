// [2.2 Characters](https://www.w3.org/TR/2006/REC-xml11-20060816/#charsets)

use crate::{
    cursor::Cursor, diagnostics::SvgParseError, file_reader::FileReader, SvgParseErrorMessage,
};
use std::iter::Peekable;

pub fn is_char(char: &char) -> bool {
    // [2]
    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = utf16[0] as u32 | (utf16[1] as u32) << 16;
    (0x1..=0xD7FF).contains(&utf16)
        || (0xE000..=0xFFFD).contains(&utf16)
        || (0x10000..=0x10FFFF).contains(&utf16)
}

pub fn is_restricted_char(char: &char) -> bool {
    // [2a]
    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = utf16[0];
    (0x1..=0x8).contains(&utf16)
        || (0xB..=0xC).contains(&utf16)
        || (0xE..=0x1F).contains(&utf16)
        || (0x7F..=0x84).contains(&utf16)
        || (0x86..=0x9F).contains(&utf16)
}

pub fn char(
    file_reader: &mut FileReader,
    expected: Option<char>,
) -> Result<(), Box<SvgParseError>> {
    let char = match file_reader.next() {
        Some(x) => x,
        None => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedEndOfFile,
        ))?,
    };

    match expected {
        Some(x) if x != char => Err(SvgParseError::new_curse(
            file_reader.get_cursor(),
            SvgParseErrorMessage::UnexpectedChar(x, char.into()),
        ))?,
        _ => {}
    };
    Ok(())
}
