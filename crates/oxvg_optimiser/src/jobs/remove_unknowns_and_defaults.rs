use lightningcss::{printer::PrinterOptions, stylesheet::ParserOptions};
use oxvg_ast::{
    atom::Atom,
    attribute::{Attr, Attributes},
    node,
    style::{Id, PresentationAttr, PresentationAttrId, Style},
    visitor::{Context, ContextFlags, PrepareOutcome},
};
use std::collections::{HashMap, HashSet};

use oxvg_ast::{document::Document, element::Element, name::Name, node::Node, visitor::Visitor};
use oxvg_collections::{
    allowed_content::ELEMS,
    collections::{Group, PRESENTATION_NON_INHERITABLE_GROUP_ATTRS},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

#[derive(Deserialize, Clone, Default, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveUnknownsAndDefaults {
    unknown_content: Option<bool>,
    unknown_attrs: Option<bool>,
    default_attrs: Option<bool>,
    default_markup_declarations: Option<bool>,
    useless_overrides: Option<bool>,
    keep_data_attrs: Option<bool>,
    keep_aria_attrs: Option<bool>,
    keep_role_attr: Option<bool>,
}

impl<E: Element> Visitor<E> for RemoveUnknownsAndDefaults {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &ContextFlags) -> PrepareOutcome {
        PrepareOutcome::use_style
    }

    fn use_style(&self, element: &E) -> bool {
        element.attributes().len() > 0
    }

    fn processing_instruction(
        &mut self,
        processing_instruction: &mut <E as oxvg_ast::node::Node>::Child,
        context: &Context<E>,
    ) -> Result<(), Self::Error> {
        if !self.default_markup_declarations.unwrap_or(true) {
            return Ok(());
        }

        let (target, data) = processing_instruction.processing_instruction().unwrap();
        let Some(mut parent) = processing_instruction.parent_node() else {
            return Ok(());
        };
        let data = PI_STANDALONE.replace(data.as_str(), "").to_string().into();
        let new_pi = context
            .root
            .as_document()
            .create_processing_instruction(target, data);
        parent.replace_child(new_pi, &processing_instruction.as_parent_child());
        Ok(())
    }

    fn element(&mut self, element: &mut E, context: &super::Context<E>) -> Result<(), String> {
        if context.flags.contains(ContextFlags::within_foreign_object) {
            return Ok(());
        }

        let name = element.qual_name();
        if name.prefix().is_some() {
            return Ok(());
        }

        self.remove_unknown_content(element);
        self.remove_unknown_and_default_attrs(element, &context.computed_styles.inherited);

        Ok(())
    }
}

impl RemoveUnknownsAndDefaults {
    const DEFAULT_KEEP_DATA_ATTRS: bool = true;
    const DEFAULT_KEEP_ARIA_ATTRS: bool = true;
    const DEFAULT_KEEP_ROLE_ATTR: bool = false;

    fn remove_unknown_content<E: Element>(&self, element: &E) {
        if !self.unknown_content.unwrap_or(true) {
            return;
        }
        let Some(parent) = Element::parent_element(element) else {
            return;
        };
        if parent.node_type() == node::Type::Document {
            return;
        }
        let parent_name = parent.qual_name().to_string();
        let name = element.qual_name().to_string();

        let allowed_children = allowed_per_element.children.get(parent_name.as_str());
        if allowed_children.is_none_or(HashSet::is_empty) {
            if !allowed_per_element.children.contains_key(name.as_str()) {
                log::debug!("removing unknown element type");
                element.remove();
            }
        } else if let Some(allowed_children) = allowed_children {
            if !allowed_children.contains(name.as_str()) {
                log::debug!("removing unknown element of parent");
                element.remove();
            }
        }
    }

    fn remove_unknown_and_default_attrs<E: Element>(
        &self,
        element: &E,
        inherited_styles: &HashMap<Id, Style>,
    ) {
        let element_name = element.qual_name().to_string();
        let allowed_attrs = allowed_per_element.attrs.get(element_name.as_str());
        let default_attrs = allowed_per_element.defaults.get(element_name.as_str());
        let has_id = element.get_attribute(&"id".into()).is_some();

        element.attributes().retain(|attr| {
            let name = attr.name();
            let local_name = name.local_name();
            log::debug!("trying to remove attr: \"{local_name}\"");
            if let Some(prefix) = name.prefix() {
                if prefix.as_str() != "xml" && prefix.as_str() != "xlink" {
                    log::debug!("ignoring prefix: {}", prefix.as_str());
                    return true;
                }
            } else {
                if self
                    .keep_data_attrs
                    .unwrap_or(Self::DEFAULT_KEEP_DATA_ATTRS)
                    && local_name.as_str().starts_with("data-")
                {
                    log::debug!("keeping data attribute");
                    return true;
                }
                if self
                    .keep_aria_attrs
                    .unwrap_or(Self::DEFAULT_KEEP_ARIA_ATTRS)
                    && local_name.as_str().starts_with("aria-")
                {
                    log::debug!("keeping aria attribute");
                    return true;
                }
                if self.keep_role_attr.unwrap_or(Self::DEFAULT_KEEP_ROLE_ATTR)
                    && local_name.as_str() == "role"
                {
                    log::debug!("keeping role attribute");
                    return true;
                }
            }

            let name_string = name.to_string();
            if let Some(allowed_attrs) = allowed_attrs {
                if self.unknown_attrs.unwrap_or(true)
                    && !allowed_attrs.contains(name_string.as_str())
                {
                    log::debug!("removing unknown attr");
                    return false;
                }
            }

            let value = attr.value();
            let inherited_value = if name.prefix().is_none() {
                let presentation_attr_id = PresentationAttrId::from(local_name.as_ref());
                if presentation_attr_id.inheritable() {
                    inherited_styles
                        .get(&Id::Attr(presentation_attr_id))
                        .or_else(|| inherited_styles.get(&Id::CSS(local_name.as_ref().into())))
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(default_attrs) = default_attrs {
                if self.default_attrs.unwrap_or(true)
                    && !has_id
                    && dbg!(default_attrs.get(name_string.as_str())) == Some(&attr.value().as_str())
                    && inherited_value.is_none()
                {
                    log::debug!(r#"removing "{local_name}" attr with default value"#);
                    return false;
                }
            }

            if self.useless_overrides.unwrap_or(true)
                && !has_id
                && !PRESENTATION_NON_INHERITABLE_GROUP_ATTRS.contains(name_string.as_str())
                && inherited_value.is_some_and(|s| {
                    if !s.is_static() {
                        log::debug!("not removing attr with inherited dynamic value");
                        return false;
                    }
                    let id = PresentationAttrId::from(local_name.as_ref());
                    let style = PresentationAttr::parse_string(
                        id,
                        value.as_ref(),
                        ParserOptions::default(),
                    );
                    if let Ok(style) = style {
                        s.inner().to_css_string(false, PrinterOptions::default())
                            == style.value_to_css_string(PrinterOptions::default()).ok()
                    } else {
                        log::debug!("not removing unknown inherited value");
                        false
                    }
                })
            {
                log::debug!("removing useless override");
                return false;
            }
            true
        });
    }
}

#[derive(Default)]
struct AllowedPerElement {
    children: HashMap<&'static str, HashSet<&'static str>>,
    attrs: HashMap<&'static str, HashSet<&'static str>>,
    defaults: HashMap<&'static str, HashMap<&'static str, &'static str>>,
}

impl AllowedPerElement {
    fn new() -> Self {
        let mut output = Self::default();
        for (key, value) in ELEMS.iter() {
            let mut allowed_children = HashSet::<&str>::new();
            if let Some(content) = &value.content {
                allowed_children.extend(content);
            }
            if let Some(content_groups) = &value.content_groups {
                for group in content_groups {
                    allowed_children.extend(group.set());
                }
            }

            let mut allowed_attrs = HashSet::<&str>::new();
            if let Some(attrs) = &value.attrs {
                allowed_attrs.extend(attrs);
            }

            let mut allowed_attr_defaults = HashMap::<&str, &str>::new();
            if let Some(defaults) = &value.defaults {
                allowed_attr_defaults.extend(defaults);
            }

            for attrs_group in &value.attrs_groups {
                allowed_attrs.extend(attrs_group.set());
                if let Some(attrs_group_defaults) = attrs_group.defaults() {
                    allowed_attr_defaults.extend(attrs_group_defaults);
                }
            }

            output.children.insert(key, allowed_children);
            output.attrs.insert(key, allowed_attrs);
            output.defaults.insert(key, allowed_attr_defaults);
        }
        output
    }
}

lazy_static! {
    static ref allowed_per_element: AllowedPerElement = AllowedPerElement::new();
    static ref PI_STANDALONE: regex::Regex =
        regex::Regex::new(r#"\s*standalone\s*=\s*["']no["']"#).unwrap();
}

#[test]
#[allow(clippy::too_many_lines)]
fn remove_unknowns_and_defaults() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:test="http://" attr="val" x="0" y="10" test:attr="val" xml:space="preserve">
    <!-- preserve xmlns and unknown prefixes -->
    <!-- preserves id'd attributes -->
    <rect fill="#000"/>
    <rect fill="#000" id="black-rect"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://">
    <!-- unknown elements are removed -->
    <test>
        test
    </test>
    <test:test>
        test
    </test:test>
    <g>
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- default values are preserved when inheritable -->
    <g fill="red">
        <path fill="#000" d="M118.8 186.9l79.2"/>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove attributes equal to inherited value -->
    <g fill="black">
        <g fill="red">
            <path fill="red" d="M118.8 186.9l79.2"/>
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove attributes equal to inherited value, excluding those with id -->
    <g fill="red">
        <g fill="red">
            <g fill="green">
                <g fill="green">
                    <path fill="red" d="M18.8 86.9l39.2"/>
                </g>
            </g>
            <path fill="red" d="M118.8 186.9l79.2"/>
            <path id="red" fill="red" d="M118.8 186.9l79.2"/>
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- allow data attributes -->
    <g fill="red" data-foo="bar">
        <path fill="#000" d="M118.8 186.9l79.2" data-bind="smth"/>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://">
    <!-- skip `foreignObject` and it's children -->
    <foreignObject>
        <div class="test">
            fallback test
        </div>
    </foreignObject>

    <test>
        test
    </test>
    <test:test>
        test
    </test:test>
    <g>
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" x="0" y="0">
    <!-- remove defaults of non-inheritable values -->
    <svg x="10" y="10">
        <svg x="0" y="0">
            <path/>
        </svg>
        <svg x="0" y="10">
            <path/>
        </svg>
        <svg x="50" y="0">
            <path/>
        </svg>
    </svg>
    <svg x="100" y="100">
        <path/>
    </svg>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove unknown elements -->
    <metadata>
        <sfw>
            <slices></slices>
            <sliceSourceBounds height="67.3" width="85.9" y="-40.8" x="-42.5" bottomLeftOrigin="true"></sliceSourceBounds>
        </sfw>
        <ellipse/>
    </metadata>
    <ellipse>
        <font-face/>
    </ellipse>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- retain matching non-inheritable attributes -->
    <g transform="translate(792)">
        <g transform="translate(792)">
            <path d="M118.8 186.9l79.2"/>
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" aria-labelledby="title">
    <!-- retain aria attributes -->
    <title id="title">
        Title
    </title>
    <g aria-label="foo">
        test
    </g>
    <path id="t" d="M10 10h10L10 20"/>
    <use href="#t"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": { "keepAriaAttrs": false } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" aria-labelledby="title">
    <!-- remove aria attrs -->
    <title id="title">
        Title
    </title>
    <g aria-label="foo">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img">
    <!-- remove default role -->
    <g/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": { "keepRoleAttr": true } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" role="img">
    <!-- retain default role -->
    <g/>
</svg>"#
        ),
    )?);

    // WARN: removes xmlns:xlink
    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg width="480" height="360" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- handle xlink and xmlns -->
    <text x="50" y="50">
        A <a xlink:href="#"><tspan>link around tspan</tspan></a> for testing
    </text>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<?xml version="1.0" standalone="no"?>
<svg width="64" height="18" xmlns="http://www.w3.org/2000/svg">
    <!-- removes `standalone="no" from xml declaration -->
    <text x="4" y="18">uwu</text>
</svg>"#
        ),
    )?);

    Ok(())
}