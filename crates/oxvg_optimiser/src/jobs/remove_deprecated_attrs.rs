use std::{cell::RefCell, collections::HashSet};

use lightningcss::{rules::CssRuleList, values::ident::Ident, visit_types, visitor::Visit};
use oxvg_ast::{
    element::Element,
    has_attribute,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::attribute::{AttrId, AttributeInfo};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

const fn default_remove_unsafe() -> bool {
    false
}

#[derive(Default, Debug)]
struct AttrStylesheet<'a> {
    names: HashSet<(Option<Ident<'a>>, Ident<'a>)>,
}

impl<'input> lightningcss::visitor::Visitor<'input> for AttrStylesheet<'input> {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(SELECTORS)
    }

    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'input>,
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

impl<'input> AttrStylesheet<'input> {
    fn extract(stylesheet: &[RefCell<CssRuleList<'input>>]) -> Result<Self, JobsError<'input>> {
        let mut result = Self::default();
        for stylesheet in stylesheet {
            stylesheet.borrow_mut().visit(&mut result)?;
        }
        Ok(result)
    }

    fn contains_qual(&self, attr: &AttrId<'input>) -> bool {
        self.names.iter().any(|(prefix, local_name)| {
            prefix.as_deref() == attr.prefix().value().as_deref()
                && **local_name == **attr.local_name()
        })
    }
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Removes deprecated attributes from elements.
///
/// # Correctnesss
///
/// By default this job should never visually change the document.
///
/// Specifying `remove_unsafe` may remove attributes which visually change
/// the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveDeprecatedAttrs {
    #[serde(default = "default_remove_unsafe")]
    /// Whether to remove deprecated presentation attributes
    pub remove_unsafe: bool,
}

impl Default for RemoveDeprecatedAttrs {
    fn default() -> Self {
        RemoveDeprecatedAttrs {
            remove_unsafe: default_remove_unsafe(),
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveDeprecatedAttrs {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        Ok(PrepareOutcome::none)
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let attributes_in_stylesheet =
            AttrStylesheet::extract(&context.query_has_stylesheet_result)?;

        // # Special cases
        // Removing deprecated xml:lang is safe when the lang attribute exists.
        if let Some(attr) = element.get_attribute(&AttrId::XmlLang) {
            if has_attribute!(element, Lang) && !attributes_in_stylesheet.contains_qual(attr.name())
            {
                drop(attr);
                element.remove_attribute(&AttrId::XmlLang);
            }
        }

        // # General cases
        self.process_attributes(element, &attributes_in_stylesheet);

        Ok(())
    }
}

impl RemoveDeprecatedAttrs {
    fn process_attributes(&self, element: &Element, attributes_in_stylesheet: &AttrStylesheet) {
        element.attributes().retain(|attr| {
            if attributes_in_stylesheet.contains_qual(attr.name()) {
                return true;
            }
            let info = attr.name().info();
            if info.contains(AttributeInfo::DeprecatedSafe)
                || (self.remove_unsafe && info.contains(AttributeInfo::DeprecatedUnsafe))
            {
                return false;
            }
            true
        });
    }
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
