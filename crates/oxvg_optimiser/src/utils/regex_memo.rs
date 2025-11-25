use std::sync::LazyLock;

use dashmap::{mapref::one::Ref, DashMap};

static MEMO: LazyLock<DashMap<String, regex::Regex>> = LazyLock::new(DashMap::new);

pub fn get(regex: &str) -> Result<Ref<'_, String, regex::Regex>, regex::Error> {
    if let Some(value) = MEMO.get(regex) {
        return Ok(value);
    }
    MEMO.insert(regex.to_string(), regex::Regex::new(regex)?);
    Ok(MEMO.get(regex).expect("Failed to assign regex memo"))
}
