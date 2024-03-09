// [2.3 Common Syntactic Constructs](https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-common-syn)

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Name;

impl Name {
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
}
