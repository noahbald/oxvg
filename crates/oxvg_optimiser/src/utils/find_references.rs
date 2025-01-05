use oxvg_collections::{
    collections::REFERENCES_PROPS,
    regex::{REFERENCES_BEGIN, REFERENCES_HREF, REFERENCES_URL},
};
use regex::CaptureMatches;

pub fn find_references<'a>(name: &str, value: &'a str) -> Option<CaptureMatches<'static, 'a>> {
    let matches = match name {
        "href" => REFERENCES_HREF.captures_iter(value),
        "begin" => REFERENCES_BEGIN.captures_iter(value),
        name if REFERENCES_PROPS.contains(name) => REFERENCES_URL.captures_iter(value),
        _ => return None,
    };
    Some(matches)
}
