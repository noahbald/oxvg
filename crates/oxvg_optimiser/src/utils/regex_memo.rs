#[cfg(not(feature = "js_sys"))]
use dashmap::{mapref::one::Ref, DashMap};
#[cfg(not(feature = "js_sys"))]
use std::sync::LazyLock;
#[cfg(feature = "js_sys")]
use std::{cell::RefCell, collections::HashMap, convert::Infallible};

#[cfg(not(feature = "js_sys"))]
#[derive(Clone)]
pub struct Regex(regex::Regex);
#[cfg(feature = "js_sys")]
#[derive(Clone)]
pub struct Regex(js_sys::RegExp);

#[cfg(not(feature = "js_sys"))]
pub type Error = regex::Error;
#[cfg(feature = "js_sys")]
pub type Error = Infallible;

#[cfg(not(feature = "js_sys"))]
static MEMO: LazyLock<DashMap<String, Regex>> = LazyLock::new(DashMap::new);
#[cfg(feature = "js_sys")]
thread_local! {
    static MEMO: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

#[cfg(not(feature = "js_sys"))]
pub fn get(regex: &str) -> Result<Ref<'_, String, Regex>, Error> {
    if let Some(value) = MEMO.get(regex) {
        return Ok(value);
    }
    MEMO.insert(regex.to_string(), Regex::new(regex)?);
    Ok(MEMO.get(regex).expect("Failed to assign regex memo"))
}

#[cfg(feature = "js_sys")]
pub(crate) struct Ref(Regex);
#[cfg(feature = "js_sys")]
impl Ref {
    pub fn value(&self) -> &Regex {
        &self.0
    }
}
#[cfg(feature = "js_sys")]
pub fn get(regex: &str) -> Result<Ref, Error> {
    MEMO.with(|memo| {
        let mut memo = memo.borrow_mut();
        if let Some(r) = memo.get(regex) {
            return Ok(Ref(r.clone()));
        }
        let r = Regex::new(regex)?;
        memo.insert(regex.to_string(), r.clone());
        Ok(Ref(r))
    })
}

impl Regex {
    #[allow(clippy::unnecessary_wraps)]
    pub fn new(regex: &str) -> Result<Self, Error> {
        #[cfg(not(feature = "js_sys"))]
        return Ok(Regex(regex::Regex::new(regex)?));
        #[cfg(feature = "js_sys")]
        return Ok(Regex(js_sys::RegExp::new(regex, "")));
    }

    pub fn is_match(&self, haystack: &str) -> bool {
        #[cfg(not(feature = "js_sys"))]
        return self.0.is_match(haystack);
        #[cfg(feature = "js_sys")]
        return self.0.test(haystack);
    }

    #[cfg(not(feature = "js_sys"))]
    #[allow(clippy::unnecessary_wraps)]
    pub fn replace(&self, haystack: &str, rep: &str) -> Result<String, Error> {
        Ok(self.0.replace(haystack, rep).to_string())
    }

    #[cfg(feature = "js_sys")]
    pub fn replace(&self, haystack: &str, rep: &str) -> Result<String, Error> {
        use std::str::FromStr as _;

        Ok(js_sys::JsString::from_str(haystack)?
            .replace_by_pattern(&self.0, rep)
            .into())
    }
}
