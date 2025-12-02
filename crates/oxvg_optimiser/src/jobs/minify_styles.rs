use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use lightningcss::{
    rules::CssRule,
    selector::Component,
    visit_types,
    visitor::{self, Visit as _, VisitTypes},
};
use oxvg_ast::{
    element::Element,
    get_attribute, get_attribute_mut, has_attribute, is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core::{Class, Id},
        AttrId,
    },
    is_prefix,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{error::JobsError, utils::minify_style};

use super::ContextFlags;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "napi", napi)]
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum RemoveUnused {
    False,
    #[default]
    True,
    Force,
    #[doc(hidden)]
    #[cfg(feature = "napi")]
    /// Compatibility option for NAPI
    // FIXME: force discriminated union to prevent NAPI from failing CI
    Napi(),
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// Minify `<style>` elements with lightningcss
///
/// # Differences to SVGO
///
/// Unlike SVGO we don't use CSSO for optimisation, instead using lightningcss.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct MinifyStyles {
    /// Whether to remove styles with no matching elements.
    #[cfg_attr(feature = "wasm", tsify(type = r#"boolean | "force""#, optional))]
    #[cfg_attr(feature = "serde", serde(default = "RemoveUnused::default"))]
    pub remove_unused: RemoveUnused,
}

#[derive(Debug)]
struct State<'a, 'input, 'arena> {
    options: &'a MinifyStyles,
    style_elements: RefCell<HashMap<usize, Element<'input, 'arena>>>,
    elements_with_style: RefCell<HashMap<usize, Element<'input, 'arena>>>,
    tags_usage: RefCell<HashSet<Atom<'input>>>,
    ids_usage: RefCell<HashSet<Id<'input>>>,
    classes_usage: RefCell<HashSet<Class<'input>>>,
}

struct StyleVisitor<'a, 'b, 'input, 'arena>(&'b State<'a, 'input, 'arena>);

impl<'input, 'arena> Visitor<'input, 'arena> for MinifyStyles {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        State::new(self).start_with_context(document, context)?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'_, 'input, 'arena> {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_script(document);
        Ok(PrepareOutcome::none)
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if is_element!(element, Style) && element.child_nodes_iter().next().is_some() {
            self.style_elements
                .borrow_mut()
                .insert(element.id(), element.clone());
        } else if has_attribute!(element, Style) {
            self.elements_with_style
                .borrow_mut()
                .insert(element.id(), element.clone());
        }

        if matches!(self.options.remove_unused, RemoveUnused::False) {
            return Ok(());
        }

        if !is_prefix!(element, SVG) {
            return Ok(());
        }
        HashSet::insert(
            &mut self.tags_usage.borrow_mut(),
            element.local_name().clone(),
        );

        if let Some(id) = get_attribute!(element, Id) {
            self.ids_usage.borrow_mut().insert(id.clone());
        }

        element
            .class_list()
            .with_iter(|iter| self.classes_usage.borrow_mut().extend(iter.cloned()));

        Ok(())
    }

    fn exit_document(
        &self,
        _document: &Element<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        for style_element in self.style_elements.borrow().values() {
            let Some(style_sheet) = style_element.child_nodes_iter().find_map(|n| n.style()) else {
                continue;
            };
            let mut style_sheet = style_sheet.borrow_mut();

            let mut visitor = StyleVisitor(self);
            match self.options.remove_unused {
                RemoveUnused::True
                    if !context
                        .flags
                        .intersects(ContextFlags::query_has_script_result) =>
                {
                    style_sheet.visit(&mut visitor)?;
                }

                RemoveUnused::Force => style_sheet.visit(&mut visitor)?,
                _ => {}
            }

            if minify_style::style_list(&mut style_sheet).is_err() {
                continue;
            }
            if style_sheet.0.is_empty() {
                log::debug!("removing empty stylesheet");
                style_element.remove();
            }
        }

        for element_with_style in self.elements_with_style.borrow().values() {
            let mut style_sheet =
                get_attribute_mut!(element_with_style, Style).expect("element without style used");

            minify_style::style(&mut style_sheet.0);
            if style_sheet.0.is_empty() {
                drop(style_sheet);
                element_with_style.remove_attribute(&AttrId::Style);
            }
        }

        Ok(())
    }
}

impl<'input> visitor::Visitor<'input> for StyleVisitor<'_, '_, 'input, '_> {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(RULES)
    }

    fn visit_rule(&mut self, rule: &mut CssRule<'input>) -> Result<(), Self::Error> {
        let CssRule::Style(style) = rule else {
            return Ok(());
        };

        let tags_usage = self.0.tags_usage.borrow();
        let ids_usage = self.0.ids_usage.borrow();
        let classes_usage = self.0.classes_usage.borrow();

        style.selectors.0.retain(|selector| {
            let iter = &mut selector.iter();
            if iter.any(|token| match token {
                Component::LocalName(name) => tags_usage.contains(&name.name.as_ref().into()),
                Component::ID(ident) => ids_usage.contains(&ident.as_ref().into()),
                Component::Class(ident) => classes_usage.contains(&ident.as_ref().into()),
                _ => true,
            }) {
                return true;
            }
            while iter.next_sequence().is_some() {
                if iter.any(|token| match token {
                    Component::LocalName(name) => tags_usage.contains(&name.name.as_ref().into()),
                    Component::ID(ident) => ids_usage.contains(&ident.as_ref().into()),
                    Component::Class(ident) => classes_usage.contains(&ident.as_ref().into()),
                    _ => true,
                }) {
                    return true;
                }
            }
            false
        });
        Ok(())
    }
}

impl<'a> State<'a, '_, '_> {
    fn new(options: &'a MinifyStyles) -> Self {
        Self {
            options,
            style_elements: RefCell::new(HashMap::new()),
            elements_with_style: RefCell::new(HashMap::new()),
            tags_usage: RefCell::new(HashSet::new()),
            ids_usage: RefCell::new(HashSet::new()),
            classes_usage: RefCell::new(HashSet::new()),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for RemoveUnused {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(match value {
            serde_json::Value::Bool(bool) => {
                if bool {
                    RemoveUnused::True
                } else {
                    RemoveUnused::False
                }
            }
            serde_json::Value::String(s) if s.as_str() == "force" => RemoveUnused::Force,
            _ => return Err(serde::de::Error::custom(r#"expected a boolean or "force""#)),
        })
    }
}

#[cfg(feature = "serde")]
impl Serialize for RemoveUnused {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RemoveUnused::True => true.serialize(serializer),
            RemoveUnused::False => false.serialize(serializer),
            RemoveUnused::Force => "force".serialize(serializer),
            #[cfg(feature = "napi")]
            Self::Napi() => panic!("Napi variant is not allowed!"),
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn minify_styles() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <style>
        .st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; } @media screen and (max-width: 200px) { .st0 { display: none; } }
    </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3; margin-top: 1em; margin-right: 1em; margin-bottom: 1em; margin-left: 1em;"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <style>
        <![CDATA[
            .st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; } @media screen and (max-width: 200px) { .st0 { display: none; } }
        ]]>
    </style>
    <style></style>
    <rect width="100" height="100" class="st0" style="stroke-width:3; margin-top: 1em; margin-right: 1em; margin-bottom: 1em; margin-left: 1em;"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <style>
        <![CDATA[
            .st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; background-image: url('data:image/svg,<svg width="16" height="16"/>') } @media screen and (max-width: 200px) { .st0 { display: none; } }
        ]]>
    </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3; margin-top: 1em; margin-right: 1em; margin-bottom: 1em; margin-left: 1em;"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <style>
        .used { p: 1 }
        .unused { p: 2 }
        #used { p: 3 }
        #unused { p: 4 }
        g { p: 5 }
        unused { p: 6 }
    </style>
    <g id="used" class="used">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": { "removeUnused": false } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <style>
        .used { p: 1 }
        .unused { p: 2 }
        #used { p: 3 }
        #unused { p: 4 }
        g { p: 5 }
        unused { p: 6 }
    </style>
    <g id="used" class="used">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <style>
        .used { p: 1 }
        .unused { p: 2 }
    </style>
    <script>
        /* script element prevents removing unused styles */
    </script>
    <g class="used">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <style>
        .used { p: 1 }
        .unused { p: 2 }
    </style>
    <g class="used" onclick="/* on* attributes prevents removing unused styles */">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": { "removeUnused": "force" } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <style>
        .used { p: 1 }
        .unused { p: 2 }
    </style>
    <script>
        /* with usage.force=true script element does not prevent removing unused styles */
    </script>
    <g class="used" onclick="/* with usage.force=true on* attributes doesn't prevent removing unused styles */">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg viewBox="0 0 2203 1777" xmlns="http://www.w3.org/2000/svg">
    <style type="text/css">
        .st6{font-family:Helvetica LT Std, Helvetica, Arial; font-size:118px;; stroke-opacity:0; fill-opacity:0;}
    </style>
    <text class="st6" transform="translate(353.67 1514)">
        tell stories in 250 characters
    </text>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 113.9 130.4">
    <style>
    .st1{fill:#453624;stroke:#453624;stroke-width:0.7495;stroke-miterlimit:10;}
    .st2{fill:#FFFFFF;}
    .st3{fill:#FCBF2A;}
    </style>
    <path d=""/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 172.87 43.39">
  <defs>
    <style>.cls-1{fill:#fff;}.cls-2{fill:#6bc49c;}</style>
  </defs>
  <g>
    <g>
      <circle class="cls-1" cx="0" cy="20" r="10"/>
      <circle class="cls-2" cx="20" cy="20" r="10"/>
      <circle class="cls-2" cx="40" cy="20" r="10"/>
    </g>
  </g>
</svg>
"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "minifyStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 50">
    <!-- preserved pseudo-classes aren't removed -->
    <style>
        :root {
            background: #fff;
        }
    </style>
</svg>"#
        ),
    )?);

    Ok(())
}
