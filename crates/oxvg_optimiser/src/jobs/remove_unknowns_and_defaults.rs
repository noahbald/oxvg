use lightningcss::{
    printer::PrinterOptions, properties::PropertyId, stylesheet::ParserOptions, traits::ToCss,
};
use oxvg_ast::{
    atom::Atom,
    attribute::{Attr, Attributes},
    style::{Id, PresentationAttr, PresentationAttrId, Style},
    visitor::{Context, ContextFlags},
};
use std::collections::{HashMap, HashSet};

use oxvg_ast::{document::Document, element::Element, name::Name, node::Node, visitor::Visitor};
use oxvg_derive::OptionalDefault;
use oxvg_selectors::{
    allowed_content::ELEMS,
    collections::{Group, PRESENTATION_NON_INHERITABLE_GROUP_ATTRS},
};
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
    keep_role_attrs: Option<bool>,
}

impl<E: Element> Visitor<E> for RemoveUnknownsAndDefaults {
    type Error = String;

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

        let Some((target, data)) = processing_instruction.processing_instruction() else {
            return Ok(());
        };
        let Some(mut parent) = processing_instruction.parent_node() else {
            return Ok(());
        };
        let data = PI_STANDALONE.replace(data.as_str(), "").to_string().into();
        let new_pi = context
            .root
            .as_document()
            .create_processing_instruction(target, data);
        parent.replace_child(processing_instruction.as_parent_child(), &new_pi);
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
    fn remove_unknown_content<E: Element>(&self, element: &E) {
        if !self.unknown_content.unwrap_or(true) {
            return;
        }
        let Some(parent) = Element::parent_element(element) else {
            return;
        };
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

        for attr in element.attributes().iter() {
            let name = attr.name();
            let local = name.local_name();
            if let Some(prefix) = name.prefix() {
                if prefix.as_str() != "xml" && prefix.as_str() != "xmlns" {
                    continue;
                }
            } else {
                if self.keep_data_attrs.unwrap_or(true) && local.as_str().starts_with("data-") {
                    continue;
                }
                if self.keep_aria_attrs.unwrap_or(true) && local.as_str().starts_with("aria-") {
                    continue;
                }
                if self.keep_role_attrs.unwrap_or(true) && local.as_str() == "role" {
                    continue;
                }
            }

            let name_string = name.to_string();
            if let Some(allowed_attrs) = allowed_attrs {
                if self.unknown_attrs.unwrap_or(true)
                    && !allowed_attrs.contains(name_string.as_str())
                {
                    log::debug!("removing unknown attr");
                    drop(attr);
                    element.remove_attribute(&name);
                    continue;
                }
            }

            let local_name = name.local_name();
            let value = attr.value();
            let local_name_when_not_pefixed = name.prefix().map(|_| local_name.as_ref());
            let inherited_value = local_name_when_not_pefixed.and_then(|n| {
                inherited_styles
                    .get(&Id::Attr(PresentationAttrId::from(n)))
                    .or_else(|| inherited_styles.get(&Id::CSS(PropertyId::from(n))))
            });
            if let Some(default_attrs) = default_attrs {
                if self.default_attrs.unwrap_or(true)
                    && !has_id
                    && default_attrs.get(name_string.as_str()) == Some(&attr.value().as_str())
                    && dbg!(inherited_value).is_none()
                {
                    log::debug!("removing attr with default value");
                    drop(attr);
                    element.remove_attribute(&name);
                    continue;
                }
            }

            if self.useless_overrides.unwrap_or(true)
                && !has_id
                && !PRESENTATION_NON_INHERITABLE_GROUP_ATTRS.contains(name_string.as_str())
                && inherited_value.is_some_and(|s| {
                    if s.is_static() {
                        return true;
                    }
                    let id = PresentationAttrId::from(local_name.as_ref());
                    let style = PresentationAttr::parse_string(
                        id,
                        value.as_ref(),
                        ParserOptions::default(),
                    );
                    if let Ok(style) = style {
                        s.inner().to_css_string(false, PrinterOptions::default())
                            == style.to_css_string(PrinterOptions::default()).ok()
                    } else {
                        false
                    }
                })
            {
                log::debug!("removing useless override");
                drop(attr);
                element.remove_attribute(&name);
                continue;
            }
        }
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
fn remove_unknowns_and_defaults() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:test="http://" attr="val" x="0" y="10" test:attr="val" xml:space="preserve">
    <rect fill="#000"/>
    <rect fill="#000" id="black-rect"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnknownsAndDefaults": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://">
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
    <g fill="red">
        <path fill="#000" d="M118.8 186.9l79.2"/>
    </g>
</svg>"##
        ),
    )?);

    Ok(())
}
