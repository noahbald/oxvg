//! A collection of regex used for matching SVG content
use regex;

lazy_static! {
    /// Matches references to elements in urls
    pub static ref REFERENCES_URL: regex::Regex =
        regex::Regex::new(r#"(?:\W|^)url\(['"]?#(.+?)['"]?\)"#).unwrap();

    /// Matches references to elements in hrefs
    pub static ref REFERENCES_HREF: regex::Regex = regex::Regex::new("^#(.+?)$").unwrap();

    /// Matches references in the `begin` attribute
    pub static ref REFERENCES_BEGIN: regex::Regex = regex::Regex::new(r"(\w+)\.[a-zA-Z]").unwrap();

    /// Matches CSS numbers
    pub static ref NUMERIC_VALUES: regex::Regex =
        regex::Regex::new(r"[-+]?(\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?").unwrap();
}
