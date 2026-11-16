use crate::{MimeType, Parser, ast::Document, error::SourceError};

fn is_void_element(name: &str) -> bool {
    [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
        "track", "wbr",
    ]
    .iter()
    .any(|v| name.eq_ignore_ascii_case(v))
}
fn is_raw_element(name: &str) -> bool {
    ["script", "style"]
        .iter()
        .any(|v| name.eq_ignore_ascii_case(v))
}
fn is_raw_escapable_element(name: &str) -> bool {
    ["textarea", "title"]
        .iter()
        .any(|v| name.eq_ignore_ascii_case(v))
}

pub fn parse<'a>(parser: Parser<'a>) -> Result<Document<'a>, SourceError> {
    debug_assert_eq!(parser.mime_type, MimeType::TextHTML);
    let Ok(source_len) = u32::try_from(parser.source.len()) else {
        return Err(SourceError::TooLong);
    };

    todo!()
}
