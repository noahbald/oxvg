use std::ops::Range;

use oxvg_collections::name::Prefix;

pub fn prefix_help(prefix: &Prefix) -> Option<String> {
    if let Prefix::Unknown { prefix, ns } = prefix {
        let prefix = prefix.as_deref();
        let ns = ns.uri();
        if let Some(prefix) = prefix {
            Some(format!(
                r#"Unknown prefix defined by `xmlns:{prefix}="{ns}"`"#
            ))
        } else {
            Some(format!(r#"Unknown prefix defined by `xmlns="{ns}"`"#))
        }
    } else {
        None
    }
}

pub fn naive_range(source: &[u8], mut start: usize) -> Range<usize> {
    while start < source.len() {
        start += 1;
        if !(source[start] as char).is_ascii_whitespace() && source[start] != b'<' {
            break;
        }
    }
    let mut end = start;
    while end < source.len() {
        if (source[end] as char).is_ascii_whitespace() || source[end] == b'>' {
            break;
        }
        end += 1;
    }
    start..end
}
