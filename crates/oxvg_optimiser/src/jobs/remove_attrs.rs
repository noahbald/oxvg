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
pub struct RemoveAttrs {
    pub attrs: Vec<String>,
    #[serde(default = "default_elem_separator")]
    pub elem_separator: String,
    #[serde(default = "default_preserve_current_color")]
    pub preserve_current_color: bool,
    #[serde(skip_deserializing, skip_serializing)]
    pub parsed_attrs: Vec<[regex::Regex; 3]>,
}

impl Default for RemoveAttrs {
    fn default() -> Self {
        RemoveAttrs {
            attrs: Default::default(),
            elem_separator: default_elem_separator(),
            preserve_current_color: default_preserve_current_color(),
            parsed_attrs: Default::default(),
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

impl<E: Element> Visitor<E> for RemoveAttrs {
    type Error = String;

    fn document(&mut self, _document: &mut E, _context: &Context<E>) -> Result<(), Self::Error> {
        let mut parsed_attrs = Vec::with_capacity(self.attrs.len());
        for pattern in &self.attrs {
            let list = self.parse_pattern(pattern).map_err(|e| e.to_string())?;
            parsed_attrs.push(list);
        }

        self.parsed_attrs = parsed_attrs;

        Ok(())
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), Self::Error> {
        for pattern in &self.parsed_attrs {
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
