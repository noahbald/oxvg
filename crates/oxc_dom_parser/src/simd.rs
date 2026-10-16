#[cfg(feature = "nightly")]
mod nightly;

use std::ops::RangeInclusive;

#[cfg(feature = "nightly")]
pub use nightly::*;

mod fallback;
#[cfg(not(feature = "nightly"))]
mod wide;

#[cfg(not(feature = "nightly"))]
use wide::{self as simd};

pub trait Op<T> {
    fn select(in_set: T) -> T;
    fn select_scalar(in_set: bool) -> bool;
}

pub fn find<const SIMD: bool, const N: usize>(haystack: &[u8], needles: [u8; N]) -> Option<usize> {
    if SIMD {
        simd::find_proto::<simd::Eq, N>(haystack, needles)
    } else {
        fallback::find_proto::<fallback::Eq, N>(haystack, needles)
    }
}

pub fn find_ne<const SIMD: bool, const N: usize>(
    haystack: &[u8],
    needles: [u8; N],
) -> Option<usize> {
    if SIMD {
        simd::find_proto::<simd::Ne, N>(haystack, needles)
    } else {
        fallback::find_proto::<fallback::Ne, N>(haystack, needles)
    }
}

pub fn find_range<const SIMD: bool, const N: usize>(
    haystack: &[u8],
    ranges: [RangeInclusive<u8>; N],
) -> Option<usize> {
    if SIMD {
        simd::find_range_proto::<simd::Eq, N>(haystack, ranges)
    } else {
        fallback::find_range_proto::<fallback::Eq, N>(haystack, ranges)
    }
}

pub fn find_range_ne<const SIMD: bool, const N: usize>(
    haystack: &[u8],
    ranges: [RangeInclusive<u8>; N],
) -> Option<usize> {
    if SIMD {
        simd::find_range_proto::<simd::Ne, N>(haystack, ranges)
    } else {
        fallback::find_range_proto::<fallback::Ne, N>(haystack, ranges)
    }
}

pub fn find_mixed<const SIMD: bool, const N: usize, const R: usize>(
    haystack: &[u8],
    needles: [u8; N],
    ranges: [RangeInclusive<u8>; R],
) -> Option<usize> {
    if SIMD {
        simd::find_mixed_proto::<simd::Eq, N, R>(haystack, needles, ranges)
    } else {
        fallback::find_mixed_proto::<(), fallback::Eq, N, R>(haystack, needles, ranges)
    }
}

pub fn find_mixed_ne<const SIMD: bool, const N: usize, const R: usize>(
    haystack: &[u8],
    needles: [u8; N],
    ranges: [RangeInclusive<u8>; R],
) -> Option<usize> {
    if SIMD {
        simd::find_mixed_proto::<simd::Ne, N, R>(haystack, needles, ranges)
    } else {
        simd::find_mixed_proto::<simd::Ne, N, R>(haystack, needles, ranges)
    }
}

const HTML_WHITESPACE: [u8; 5] = [b'\t', b'\n', b' ', b'\r', b'\x0C'];
pub fn find_html_non_whitespace<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find_ne::<SIMD, _>(haystack, HTML_WHITESPACE)
}
pub fn find_html_whitespace<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find::<SIMD, _>(haystack, HTML_WHITESPACE)
}
const HTML_WHITESPACE_OR_CLOSE: [u8; 6] = [b'\t', b'\n', b' ', b'\r', b'\x0C', b'>'];
pub fn find_html_whitespace_or_close<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find::<SIMD, _>(haystack, HTML_WHITESPACE_OR_CLOSE)
}

pub fn is_html_name_start(haystack: &[u8]) -> bool {
    haystack.first().is_some_and(u8::is_ascii_alphabetic)
}

const HTML_NAME_RANGES: [RangeInclusive<u8>; 3] = [b'0'..=b'9', b'a'..=b'z', b'A'..=b'Z'];
pub fn find_html_tag_name_end<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find_mixed_ne::<SIMD, _, _>(haystack, [b'-', b'_', b'='], HTML_NAME_RANGES)
}
pub fn find_html_attr_name_end<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find_mixed_ne::<SIMD, _, _>(haystack, [b'-', b'_'], HTML_NAME_RANGES)
}

const XML_WHITESPACE: [u8; 4] = [b'\t', b'\n', b' ', b'\r'];
pub fn find_xml_non_whitespace<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find_ne::<SIMD, _>(haystack, XML_WHITESPACE)
}
pub fn find_xml_whitespace<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find::<SIMD, _>(haystack, XML_WHITESPACE)
}
const XML_WHITESPACE_OR_CLOSE: [u8; 5] = [b'\t', b'\n', b' ', b'\r', b'>'];
pub fn find_xml_whitespace_or_close<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    find::<SIMD, _>(haystack, XML_WHITESPACE_OR_CLOSE)
}

const NON_ASCII: RangeInclusive<u8> = 0x80..=0xFF;
pub fn is_xml_name_start(haystack: &[u8]) -> bool {
    match haystack.first() {
        Some(b) if b.is_ascii() => b.is_ascii_alphabetic() || matches!(*b, b'_' | b':'),
        Some(_) => decode_char(haystack).is_some_and(|(c, _)| is_xml_name_start_char(c)),
        None => false,
    }
}

fn is_xml_name_start_char(c: char) -> bool {
    matches!(c as u32,
        0x0000C0..=0x0000D6
        | 0x0000D8..=0x0000F6
        | 0x0000F8..=0x0002FF
        | 0x000370..=0x00037D
        | 0x00037F..=0x001FFF
        | 0x00200C..=0x00200D
        | 0x002070..=0x00218F
        | 0x002C00..=0x002FEF
        | 0x003001..=0x00D7FF
        | 0x00F900..=0x00FDCF
        | 0x00FDF0..=0x00FFFD
        | 0x010000..=0x0EFFFF)
}

pub fn find_xml_name_end<const SIMD: bool>(haystack: &[u8]) -> Option<usize> {
    let mut offset = 0;
    loop {
        let rest = &haystack[offset..];
        let pos = offset
            + find_mixed_ne::<SIMD, _, _>(
                rest,
                [b'_', b':', b'-', b'.'],
                [b'0'..=b'9', b'a'..=b'z', b'A'..=b'Z'],
            )?;

        // Check for Unicode char and continue if neeeded
        if haystack[pos] < 0x80 {
            return Some(pos);
        }
        match decode_char(&haystack[pos..]) {
            Some((c, len)) if is_xml_name_char(c) => offset = pos + len,
            _ => return Some(pos),
        }
    }
}

fn is_xml_name_char(c: char) -> bool {
    is_xml_name_start_char(c)
        || matches!(c as u32, 0x0000B7 | 0x000300..=0x00036F | 0x00203F..=0x002040)
}

fn decode_char(bytes: &[u8]) -> Option<(char, usize)> {
    let len = match *bytes.first()? {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => return None,
    };
    let c = std::str::from_utf8(bytes.get(..len)?)
        .ok()?
        .chars()
        .next()?;
    Some((c, len))
}
