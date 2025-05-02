use std::sync::OnceLock;

use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

fn default_elem_separator() -> String {
    String::from(":")
}

const fn default_preserve_current_color() -> bool {
    false
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Remove attributes based on whether it matches a pattern.
///
/// The patterns syntax is `[ element* : attribute* : value* ]`; where
///
/// - A regular expression matching an element's name. An asterisk or omission matches all.
/// - A regular expression matching an attribute's name.
/// - A regular expression matching an attribute's value. An asterisk or omission matches all.
///
/// # Example
///
/// Match `fill` attribute in `<path>` elements
///
/// ```
/// use oxvg_optimiser::{Jobs, RemoveAttrs};
///
/// let mut remove_attrs = RemoveAttrs::default();
/// remove_attrs.attrs = vec![String::from("path:fill")];
/// let jobs = Jobs {
///   remove_attrs: Some(remove_attrs),
///   ..Jobs::none()
/// };
/// ```
/// # Correctness
///
/// Removing attributes may visually change the document if they're
/// presentation attributes or selected with CSS.
///
/// # Errors
///
/// If the regex fails to parse.
pub struct RemoveAttrs {
    // FIXME: We really don't need the complexity of a DSL here.
    /// A list of patterns that match attributes.
    pub attrs: Vec<String>,
    #[serde(default = "default_elem_separator")]
    /// The seperator for different parts of the pattern. By default this is `":"`.
    ///
    /// You may need to use this if you need to match attributes with a `:` (i.e. prefixed attributes).
    pub elem_separator: String,
    #[serde(default = "default_preserve_current_color")]
    /// Whether to ignore attributes set to `currentColor`
    pub preserve_current_color: bool,
    // FIXME: Can't produce napi bindgen
    // https://github.com/napi-rs/napi-rs/issues/2582
    #[serde(skip_deserializing, skip_serializing)]
    parsed_attrs_memo: OnceLock<Result<Vec<[regex::Regex; 3]>, String>>,
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::TypeName for RemoveAttrs {
    fn type_name() -> &'static str {
        "RemoveAttrs"
    }
    fn value_type() -> napi::ValueType {
        napi::ValueType::Object
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ToNapiValue for RemoveAttrs {
    unsafe fn to_napi_value(
        env: napi::bindgen_prelude::sys::napi_env,
        val: RemoveAttrs,
    ) -> napi::bindgen_prelude::Result<napi::bindgen_prelude::sys::napi_value> {
        let env_wrapper = napi::bindgen_prelude::Env::from(env);
        let mut obj = env_wrapper.create_object()?;
        obj.set("attrs", val.attrs)?;
        obj.set("elemSeparator", val.elem_separator)?;
        obj.set("preserveCurrentColor", val.preserve_current_color)?;
        napi::bindgen_prelude::Object::to_napi_value(env, obj)
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::FromNapiValue for RemoveAttrs {
    unsafe fn from_napi_value(
        env: napi::bindgen_prelude::sys::napi_env,
        napi_val: napi::bindgen_prelude::sys::napi_value,
    ) -> napi::bindgen_prelude::Result<RemoveAttrs> {
        let obj = napi::bindgen_prelude::Object::from_napi_value(env, napi_val)?;
        let attrs: Vec<String> = obj
            .get("attrs")
            .map_err(|mut err| {
                err.reason = format!("{} on RemoveAttrs.attrs", err.reason);
                err
            })?
            .ok_or_else(|| napi::Error::new(napi::Status::InvalidArg, "Missing field `attrs`"))?;
        let elem_separator: String = obj
            .get("elemSeparator")
            .map_err(|mut err| {
                err.reason = format!("{} on RemoveAttrs.elemSeparator", err.reason);
                err
            })?
            .ok_or_else(|| {
                napi::Error::new(napi::Status::InvalidArg, "Missing field `elemSeparator`")
            })?;
        let preserve_current_color: bool = obj
            .get("preserveCurrentColor")
            .map_err(|mut err| {
                err.reason = format!("{} on RemoveAttrs.preserveCurrentColor", err.reason);
                err
            })?
            .ok_or_else(|| {
                napi::Error::new(
                    napi::Status::InvalidArg,
                    "Missing field `preserveCurrentColor`",
                )
            })?;
        let val = Self {
            attrs,
            elem_separator,
            preserve_current_color,
            parsed_attrs_memo: OnceLock::default(),
        };
        Ok(val)
    }
}
#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ValidateNapiValue for RemoveAttrs {}

impl Default for RemoveAttrs {
    fn default() -> Self {
        RemoveAttrs {
            attrs: Vec::default(),
            elem_separator: default_elem_separator(),
            preserve_current_color: default_preserve_current_color(),
            parsed_attrs_memo: OnceLock::default(),
        }
    }
}

fn create_regex(part: &str) -> Result<regex::Regex, regex::Error> {
    if matches!(part, "*" | ".*") {
        return Ok(WILDCARD.clone());
    }
    regex::Regex::new(&format!("^{part}$"))
}

impl RemoveAttrs {
    fn parse_pattern(&self, pattern: &str) -> Result<[regex::Regex; 3], regex::Error> {
        let list = match pattern.split_once(&self.elem_separator) {
            Some((start, rest)) => match rest.split_once(&self.elem_separator) {
                Some((middle, end)) => [
                    create_regex(start)?,
                    create_regex(middle)?,
                    create_regex(end)?,
                ],
                None => [create_regex(start)?, create_regex(rest)?, WILDCARD.clone()],
            },
            None => [WILDCARD.clone(), create_regex(pattern)?, WILDCARD.clone()],
        };
        Ok(list)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveAttrs {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        let parsed_attrs = self.parsed_attrs_memo.get_or_init(|| {
            let mut parsed_attrs = Vec::with_capacity(self.attrs.len());
            for pattern in &self.attrs {
                let list = self.parse_pattern(pattern).map_err(|e| e.to_string())?;
                parsed_attrs.push(list);
            }
            Ok(parsed_attrs)
        });
        let parsed_attrs = match parsed_attrs {
            Ok(a) => a,
            Err(e) => return Err(e.clone()),
        };
        for pattern in parsed_attrs {
            if !pattern[0].is_match(&element.qual_name().formatter().to_string()) {
                continue;
            }

            element.attributes().retain(|attr| {
                let is_current_color = self.preserve_current_color
                    && attr.prefix().is_none()
                    && matches!(attr.local_name().as_ref(), "fill" | "stroke")
                    && attr.value().as_ref().to_lowercase() == "currentcolor";
                if is_current_color {
                    return true;
                }

                if !pattern[2].is_match(attr.value().as_ref()) {
                    return true;
                }

                let name = attr.name().formatter().to_string();
                !pattern[1].is_match(&name)
            });
        }

        Ok(())
    }
}

lazy_static! {
    static ref WILDCARD: regex::Regex = regex::Regex::new(".*").unwrap();
}

#[test]
fn remove_attrs() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeAttrs": {
            "attrs": ["circle:stroke.*", "path:fill"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <circle fill="red" stroke-width="6" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <circle stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <path fill="red" stroke="red" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeAttrs": {
            "attrs": ["(fill|stroke)"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <circle fill="red" stroke-width="6" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <circle stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <path fill="red" stroke="red" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeAttrs": {
            "attrs": ["(fill|stroke)"],
            "preserveCurrentColor": true
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <circle fill="currentColor" stroke-width="6" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <circle stroke="currentColor" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <path fill="red" stroke="red" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeAttrs": {
            "attrs": ["*:(stroke|fill):red"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <circle fill="red" stroke-width="6" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <circle stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <circle stroke="#FFF" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="25"/>
    <path fill="red" stroke="red" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeAttrs": {
            "attrs": ["fill"],
            "preserveCurrentColor": true
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 150 150">
    <linearGradient id="A">
        <stop stop-color="ReD" offset="5%"/>
    </linearGradient>
    <text x="0" y="32" fill="currentColor">uwu</text>
    <text x="0" y="64" fill="currentcolor">owo</text>
    <text x="0" y="96" fill="url(#A)">eue</text>
</svg>"#
        )
    )?);

    Ok(())
}
