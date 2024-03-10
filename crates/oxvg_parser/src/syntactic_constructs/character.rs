/// Returns whether the character is a valid character for an XML document.
///
/// As per [2.2 Characters](https://www.w3.org/TR/2006/REC-xml11-20060816/#charsets),
/// "any Unicode character, excluding the surrogate blocks, FFFE, and FFFF."^[2](https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-Char)
pub fn is(char: char) -> bool {
    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = u32::from(utf16[0]) | u32::from(utf16[1]) << 16;
    (0x1..=0xD7FF).contains(&utf16)
        || (0xE000..=0xFFFD).contains(&utf16)
        || (0x10000..=0x0010_FFFF).contains(&utf16)
}

/// Returns whether the character is a restricter character, that is, characters disallowed as part
/// of and within the root element
///
/// As per [2.2 Characters](https://www.w3.org/TR/2006/REC-xml11-20060816/#charsets)
/// ^[2a](https://www.w3.org/TR/2006/REC-xml11-20060816/#NT-RestrictedChar)
pub fn is_restricted(char: char) -> bool {
    let mut utf16 = [0; 2];
    char.encode_utf16(&mut utf16);
    let utf16 = utf16[0];
    (0x1..=0x8).contains(&utf16)
        || (0xB..=0xC).contains(&utf16)
        || (0xE..=0x1F).contains(&utf16)
        || (0x7F..=0x84).contains(&utf16)
        || (0x86..=0x9F).contains(&utf16)
}
