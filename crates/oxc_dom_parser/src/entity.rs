use std::collections::HashMap;

use oxc_allocator::GetAllocator;

use crate::{
    Error, ErrorKind, Span,
    ast::{EscapedStr, Name, TextSpan},
};

#[derive(Default)]
pub enum Entities<'a, const HTML: bool> {
    #[default]
    Default,
    Declared(HashMap<&'a str, &'a str>),
}

impl<'a, const HTML: bool> Entities<'a, HTML> {
    fn find_special<const IS_ATTR: bool>(bytes: &[u8]) -> Option<usize> {
        if IS_ATTR && !HTML {
            crate::simd::find::<true, _>(bytes, [b'&', b'\r', b'\t', b'\n', b'<'])
        } else {
            crate::simd::find::<true, _>(bytes, [b'&', b'\r'])
        }
    }

    pub fn resolve<const Attribute: bool>(
        &self,
        original: TextSpan<'a>,
        scratch: &mut String,
        allocator: &impl GetAllocator<'a>,
    ) -> Result<EscapedStr<'a>, Error> {
        let source = original.value.as_str();
        let bytes = source.as_bytes();
        let Some(mut pos) = Self::find_special::<Attribute>(bytes) else {
            return Ok(EscapedStr::new(source, source));
        };

        let base = original.span.start;
        let error = |kind: ErrorKind, start: usize, end: usize| {
            Error::new(kind, Span::new(base + start as u32, base + end as u32))
        };
        let normalise_whitespace = Attribute && !HTML;

        scratch.clear();
        scratch.reserve(source.len());
        let mut chunk_start = 0;

        loop {
            let mut next = pos + 1;
            match bytes[pos] {
                b'&' => match source[pos + 1..].find(';') {
                    None if !HTML => return Err(error(ErrorKind::InvalidReference, pos, pos + 1)),
                    // HTML: a bare `&` is text. TODO: name-end scan + 106 semicolon-less names
                    None => {}
                    Some(semi) => {
                        let name = &source[pos + 1..pos + 1 + semi];
                        let end = pos + semi + 2; // one past `;`
                        scratch.push_str(&source[chunk_start..pos]);

                        let resolved = if let Some(digits) = name.strip_prefix('#') {
                            decode_char_reference(digits)
                                .map(|c| scratch.push(c))
                                .ok_or(ErrorKind::InvalidReference)
                        } else if !Name::is_name::<HTML, Attribute>(name) {
                            Err(ErrorKind::InvalidReference)
                        } else {
                            self.push(name, scratch)
                                .map_err(|()| ErrorKind::UnknownReference)
                        };

                        match resolved {
                            Ok(()) => {
                                chunk_start = end;
                                next = end;
                            }
                            Err(kind) if !HTML => return Err(error(kind, pos, end)),
                            // HTML: unresolvable reference, keep `&` literally and rescan after it
                            Err(_) => {
                                scratch.push('&');
                                chunk_start = pos + 1;
                            }
                        }
                    }
                },
                b'\r' => {
                    scratch.push_str(&source[chunk_start..pos]);
                    scratch.push(if normalise_whitespace { ' ' } else { '\n' });
                    if bytes.get(pos + 1) == Some(&b'\n') {
                        next += 1;
                    }
                    chunk_start = next;
                }
                b'\t' | b'\n' if normalise_whitespace => {
                    scratch.push_str(&source[chunk_start..pos]);
                    scratch.push(' ');
                    chunk_start = next;
                }
                b'<' if normalise_whitespace => {
                    return Err(error(ErrorKind::InvalidAttributeCharacter, pos, pos + 1));
                }
                _ => {}
            }

            match Self::find_special::<Attribute>(&bytes[next..]) {
                Some(i) => pos = next + i,
                None => break,
            }
        }

        scratch.push_str(&source[chunk_start..]);
        Ok(EscapedStr::new_in(
            original.value,
            scratch.as_str(),
            allocator,
        ))
    }

    pub fn register(&mut self, name: &'a str, value: &'a str) {
        if let Self::Declared(set) = self {
            set.insert(name, value);
        } else {
            *self = Self::Declared([(name, value)].into())
        }
    }

    pub fn push(&self, name: &str, scratch: &mut String) -> Result<(), ()> {
        let default = match name {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "apos" => Some('\''),
            "quot" => Some('"'),
            _ => None,
        };
        if let Some(char) = default {
            scratch.push(char);
            return Ok(());
        }
        #[cfg(feature = "html")]
        if HTML {
            todo!()
        }
        match self {
            Self::Default => Err(()),
            Self::Declared(set) => {
                if let Some(reference) = set.get(name) {
                    scratch.push_str(reference);
                    Ok(())
                } else {
                    Err(())
                }
            }
        }
    }
}

fn decode_char_reference(digits: &str) -> Option<char> {
    let (radix, digits) = match digits.strip_prefix('x') {
        Some(hex) => (16, hex),
        None => (10, digits),
    };
    // `from_str_radix` accepts a leading `+`, so check the characters ourselves
    if digits.is_empty() || !digits.bytes().all(|b| (b as char).is_digit(radix)) {
        return None;
    }
    let c = char::from_u32(u32::from_str_radix(digits, radix).ok()?)?;
    matches!(
        c,
        '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
    )
    .then_some(c)
}
