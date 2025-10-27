use std::{cell::RefCell, collections::HashSet};

use lightningcss::{
    printer::PrinterOptions,
    rules::CssRule,
    selector::Component,
    stylesheet::{MinifyOptions, ParserOptions, StyleAttribute, StyleSheet},
    traits::ToCss as _,
    visit_types,
    visitor::{self, Visit as _, VisitTypes},
};
use oxvg_ast::{
    attribute::Attr as _,
    class_list::ClassList as _,
    element::Element,
    name::Name,
    visitor::{Context, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

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
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
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
    #[serde(default = "RemoveUnused::default")]
    pub remove_unused: RemoveUnused,
}

#[derive(Debug)]
struct State<'a, 'arena, E: Element<'arena>> {
    options: &'a MinifyStyles,
    style_elements: RefCell<HashSet<E>>,
    elements_with_style: RefCell<HashSet<E>>,
    tags_usage: RefCell<HashSet<<E::Name as Name>::LocalName>>,
    ids_usage: RefCell<HashSet<E::Atom>>,
    classes_usage: RefCell<HashSet<E::Atom>>,
}

struct StyleVisitor<'a, 'b, 'arena, E: Element<'arena>>(&'b State<'a, 'arena, E>);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for MinifyStyles {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        State::new(self).start(&mut document.clone(), info, None)?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'_, 'arena, E> {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        _info: &Info<'arena, E>,
        context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        context_flags.query_has_script(document);
        Ok(PrepareOutcome::none)
    }

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if element.local_name().as_ref() == "style" && element.child_nodes_iter().next().is_some() {
            self.style_elements.borrow_mut().insert(element.clone());
        } else if element.has_attribute_local(&"style".into()) {
            self.elements_with_style
                .borrow_mut()
                .insert(element.clone());
        }

        if matches!(self.options.remove_unused, RemoveUnused::False) {
            return Ok(());
        }

        self.tags_usage
            .borrow_mut()
            .insert(element.local_name().clone());

        if let Some(id) = element.get_attribute_local(&"id".into()) {
            self.ids_usage.borrow_mut().insert(id.clone());
        }

        for class in element.class_list().iter() {
            self.classes_usage.borrow_mut().insert(class.clone());
        }

        Ok(())
    }

    fn exit_document(
        &self,
        _document: &mut E,
        context: &Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        for style_element in self.style_elements.borrow().iter() {
            let css_text = style_element
                .text_content()
                .expect("non-style element used");

            let Ok(mut style_sheet) = StyleSheet::parse(&css_text, ParserOptions::default()) else {
                continue;
            };

            let mut visitor = StyleVisitor(self);
            match self.options.remove_unused {
                RemoveUnused::True if !context.flags.intersects(ContextFlags::has_script_ref) => {
                    style_sheet.visit(&mut visitor)?;
                }

                RemoveUnused::Force => style_sheet.visit(&mut visitor)?,
                _ => {}
            }

            if style_sheet.minify(MinifyOptions::default()).is_err() {
                continue;
            }
            let Ok(minified) = style_sheet.rules.to_css_string(PrinterOptions {
                minify: true,
                ..PrinterOptions::default()
            }) else {
                continue;
            };

            if minified.is_empty() {
                style_element.remove();
            } else {
                style_element.set_text_content(minified.into(), &context.info.arena);
            }
        }

        for element_with_style in self.elements_with_style.borrow().iter() {
            let mut style_attr = element_with_style
                .get_attribute_node_local_mut(&"style".into())
                .expect("element without style used");

            let Ok(mut style_sheet) =
                StyleAttribute::parse(style_attr.value(), ParserOptions::default())
            else {
                continue;
            };
            style_sheet.minify(MinifyOptions::default());
            let Ok(minified) = style_sheet.declarations.to_css_string(PrinterOptions {
                minify: true,
                ..PrinterOptions::default()
            }) else {
                continue;
            };
            drop(style_sheet);
            let minified = minified.into();
            style_attr.set_value(minified);
        }

        Ok(())
    }
}

impl<'i, 'arena, E: Element<'arena>> visitor::Visitor<'i> for StyleVisitor<'_, '_, 'arena, E> {
    type Error = String;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(RULES)
    }

    fn visit_rule(&mut self, rule: &mut CssRule<'i>) -> Result<(), Self::Error> {
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

impl<'a, 'arena, E: Element<'arena>> State<'a, 'arena, E> {
    fn new(options: &'a MinifyStyles) -> Self {
        Self {
            options,
            style_elements: RefCell::new(HashSet::new()),
            elements_with_style: RefCell::new(HashSet::new()),
            tags_usage: RefCell::new(HashSet::new()),
            ids_usage: RefCell::new(HashSet::new()),
            classes_usage: RefCell::new(HashSet::new()),
        }
    }
}

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
