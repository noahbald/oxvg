use std::collections::HashSet;

use lightningcss::{stylesheet::StyleSheet, values::ident::Ident, visit_types};
use oxvg_ast::{
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    allowed_content::{attrs_group_deprecated_unsafe, ELEMS},
    collections::CORE,
};
use serde::{Deserialize, Serialize};

const fn default_remove_unsafe() -> bool {
    false
}

#[derive(Default)]
struct AttrStylesheet<'a> {
    names: HashSet<(Option<Ident<'a>>, Ident<'a>)>,
}

impl<'a> lightningcss::visitor::Visitor<'a> for AttrStylesheet<'a> {
    type Error = String;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(SELECTORS)
    }

    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'a>,
    ) -> Result<(), Self::Error> {
        use parcel_selectors::attr::NamespaceConstraint;
        use parcel_selectors::parser::Component;

        let local_names = selector.iter_raw_match_order().filter_map(|c| match c {
            Component::AttributeInNoNamespaceExists {
                local_name_lower: local_name,
                ..
            }
            | Component::AttributeInNoNamespace { local_name, .. } => {
                Some((None, local_name.clone()))
            }
            Component::AttributeOther(other) => match other.namespace {
                Some(NamespaceConstraint::Any) | None => Some((None, other.local_name.clone())),
                Some(NamespaceConstraint::Specific((ref prefix, _))) => {
                    Some((Some(prefix.clone()), other.local_name.clone()))
                }
            },
            _ => None,
        });
        self.names.extend(local_names);
        Ok(())
    }
}

impl<'a> AttrStylesheet<'a> {
    fn extract(stylesheet: &mut StyleSheet<'a, '_>) -> Result<Self, String> {
        use lightningcss::visitor::Visitor;

        let mut result = Self::default();
        result.visit_stylesheet(stylesheet)?;
        Ok(result)
    }

    fn contains_qual(&self, prefix: Option<&str>, local_name: &str) -> bool {
        self.names
            .iter()
            .any(|n| n.0.as_ref().map(AsRef::as_ref) == prefix && n.1.as_ref() == local_name)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoveDeprecatedAttrs {
    #[serde(default = "default_remove_unsafe")]
    pub remove_unsafe: bool,
}

impl Default for RemoveDeprecatedAttrs {
    fn default() -> Self {
        RemoveDeprecatedAttrs {
            remove_unsafe: default_remove_unsafe(),
        }
    }
}

impl<E: Element> Visitor<E> for RemoveDeprecatedAttrs {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        PrepareOutcome::use_style
    }

    fn element(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), Self::Error> {
        let Some(elem_config) = ELEMS.get(element.qual_name().formatter().to_string().as_str())
        else {
            return Ok(());
        };

        let attributes_in_stylesheet = match &mut context.stylesheet {
            Some(ref mut stylesheet) => AttrStylesheet::extract(stylesheet)?,
            None => AttrStylesheet::default(),
        };

        // # Special cases
        // Removing deprecated xml:lang is safe when the lang attribute exists.
        if elem_config
            .attrs_groups
            .iter()
            .any(|m| m.map.key == CORE.map.key)
        {
            log::debug!("removing lang from core element");
            let xml_lang_name = E::Name::new(Some("xml".into()), "lang".into());
            if element.has_attribute(&xml_lang_name)
                && element.has_attribute_local(&"lang".into())
                && !attributes_in_stylesheet.contains_qual(Some("xml"), "lang")
            {
                element.remove_attribute(&xml_lang_name);
            }
        }

        // # General cases
        elem_config.attrs_groups.iter().for_each(|group| {
            self.process_attributes(
                element,
                None,
                attrs_group_deprecated_unsafe(group),
                &attributes_in_stylesheet,
            );
        });

        self.process_attributes(
            element,
            elem_config.deprecated_safe.as_ref(),
            elem_config.deprecated_unsafe.as_ref(),
            &attributes_in_stylesheet,
        );

        Ok(())
    }
}

impl RemoveDeprecatedAttrs {
    fn process_attributes<E: Element>(
        &self,
        element: &E,
        group_deprecated_safe: Option<&phf::Set<&'static str>>,
        group_deprecated_unsafe: Option<&phf::Set<&'static str>>,
        attributes_in_stylesheet: &AttrStylesheet,
    ) {
        if let Some(deprecated) = group_deprecated_safe {
            for deprecated in deprecated {
                let (prefix, local_name) = match deprecated.split_once(':') {
                    Some((prefix, local_name)) => (Some(prefix), local_name),
                    None => (None, *deprecated),
                };
                if attributes_in_stylesheet.contains_qual(prefix, local_name) {
                    continue;
                }
                element.remove_attribute(&E::Name::new(prefix.map(Into::into), local_name.into()));
            }
        }

        if self.remove_unsafe {
            if let Some(deprecated) = group_deprecated_unsafe {
                for deprecated in deprecated {
                    let (prefix, local_name) = match deprecated.split_once(':') {
                        Some((prefix, local_name)) => (Some(prefix), local_name),
                        None => (None, *deprecated),
                    };
                    if attributes_in_stylesheet.contains_qual(prefix, local_name) {
                        continue;
                    }
                    element
                        .remove_attribute(&E::Name::new(prefix.map(Into::into), local_name.into()));
                }
            }
        }
    }
}

lazy_static! {
    static ref WILDCARD: regex::Regex = regex::Regex::new(".*").unwrap();
}

#[test]
fn remove_attrs() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": {} }"#,
        Some(
            r#"<svg version="1.1" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <!-- removes deprecated `version` -->
    <rect x="10" y="10" width="80" height="80"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": {} }"#,
        Some(
            r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <!-- leaves unsafe to remove deprecated `viewTarget` -->
    <view id="one" viewBox="0 0 100 100" viewTarget=""/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": { "removeUnsafe": true } }"#,
        Some(
            r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <!-- removes unsafe to remove deprecated `viewTarget` -->
    <view id="one" viewBox="0 0 100 100" viewTarget=""/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": { "removeUnsafe": true } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100.5" height=".5" enable-background="new 0 0 100.5 .5">
    <!-- remove deprecated `enable-background` -->
    <defs>
        <filter id="ShiftBGAndBlur">
            <feOffset dx="0" dy="75"/>
        </filter>
    </defs>
    test
</svg>"#
        )
    )?);

    // FIXME: markup5ever removes `lang`
    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": {} }"#,
        Some(
            r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <!-- remove deprecated `xml:lang` in presence of `lang` -->
    <text xml:lang="en-CA" lang="en-US">English text</text>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": {} }"#,
        Some(
            r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <!-- keeps `xml:lang` when standalone -->
    <text xml:lang="en-US">English text</text>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": { "removeUnsafe": true } }"#,
        Some(
            r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <!-- removes unsafe to remove deprecated `xml:lang` -->
    <text xml:lang="en-US">English text</text>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDeprecatedAttrs": { "removeUnsafe": true } }"#,
        Some(
            r#"<svg version="1.1" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <!-- keep selected `version` -->
    <style>
        <![CDATA[svg[version="1.1"]{fill:blue;}rect[clip]{fill:green;}]]>
    </style>
    <rect x="10" y="10" width="80" height="80" clip="1"/>
</svg>"#
        )
    )?);

    Ok(())
}
