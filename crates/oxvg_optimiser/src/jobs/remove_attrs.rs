use std::sync::LazyLock;

use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use oxvg_collections::{
    attribute::{
        core::{Color, Paint},
        inheritable::Inheritable,
    },
    content_type::{ContentType, ContentTypeRef},
};
use oxvg_serialize::{PrinterOptions, ToValue as _};
use serde::{Deserialize, Serialize};

fn default_elem_separator() -> String {
    String::from(":")
}

const fn default_preserve_current_color() -> bool {
    false
}

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::{error::JobsError, utils::regex_memo};

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
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
    /// The separator for different parts of the pattern. By default this is `":"`.
    ///
    /// You may need to use this if you need to match attributes with a `:` (i.e. prefixed attributes).
    pub elem_separator: String,
    #[serde(default = "default_preserve_current_color")]
    /// Whether to ignore attributes set to `currentColor`
    pub preserve_current_color: bool,
}

impl Default for RemoveAttrs {
    fn default() -> Self {
        RemoveAttrs {
            attrs: Vec::default(),
            elem_separator: default_elem_separator(),
            preserve_current_color: default_preserve_current_color(),
        }
    }
}

fn create_regex(part: &str) -> Result<regex::Regex, regex::Error> {
    if matches!(part, "*" | ".*") {
        return Ok(WILDCARD.clone());
    }
    regex_memo::get(&format!("^{part}$")).map(|memo| memo.value().clone())
}

impl RemoveAttrs {
    fn parse_pattern<'input>(&self, pattern: &str) -> Result<[regex::Regex; 3], JobsError<'input>> {
        let list = match pattern.split_once(&self.elem_separator) {
            Some((start, rest)) => match rest.split_once(&self.elem_separator) {
                Some((middle, end)) => [
                    create_regex(start).map_err(JobsError::InvalidUserRegex)?,
                    create_regex(middle).map_err(JobsError::InvalidUserRegex)?,
                    create_regex(end).map_err(JobsError::InvalidUserRegex)?,
                ],
                None => [
                    create_regex(start).map_err(JobsError::InvalidUserRegex)?,
                    create_regex(rest).map_err(JobsError::InvalidUserRegex)?,
                    WILDCARD.clone(),
                ],
            },
            None => [
                WILDCARD.clone(),
                create_regex(pattern).map_err(JobsError::InvalidUserRegex)?,
                WILDCARD.clone(),
            ],
        };
        Ok(list)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveAttrs {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut parsed_attrs = Vec::with_capacity(self.attrs.len());
        for pattern in &self.attrs {
            let list = self.parse_pattern(pattern)?;
            parsed_attrs.push(list);
        }
        for pattern in parsed_attrs {
            if !pattern[0].is_match(&element.qual_name().to_string()) {
                continue;
            }

            element.attributes().retain(|attr| {
                if self.preserve_current_color
                    && matches!(
                        attr.value(),
                        ContentType::Inheritable(Inheritable::Defined(attr)) if matches!(*attr, ContentType::Paint(
                            ContentTypeRef::Ref(&Paint::Color(Color::CurrentColor)),
                        ))
                    )
                {
                    return true;
                }

                if !pattern[2].is_match(
                    &attr
                        .value()
                        .to_value_string(PrinterOptions::default())
                        .unwrap(),
                ) {
                    return true;
                }

                let name = attr.name().to_string();
                !pattern[1].is_match(&name)
            });
        }

        Ok(())
    }
}

static WILDCARD: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(".*").unwrap());

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
