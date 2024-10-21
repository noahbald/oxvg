use regex;

lazy_static! {
    pub static ref REFERENCES_URL: regex::Regex =
        regex::Regex::new(r#"(?:\W|^)url\(['"]?#(.+?)['"]?\)"#).unwrap();
    pub static ref REFERENCES_HREF: regex::Regex = regex::Regex::new("^#(.+?)$").unwrap();
    pub static ref REFERENCES_BEGIN: regex::Regex = regex::Regex::new(r"(\w+)\.[a-zA-Z]").unwrap();
    pub static ref NUMERIC_VALUES: regex::Regex =
        regex::Regex::new(r"[-+]?(\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?").unwrap();
}
