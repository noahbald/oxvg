use lightningcss::{
    printer::PrinterOptions,
    rules::CssRuleList,
    stylesheet::{MinifyOptions, ParserFlags, ParserOptions, StyleAttribute, StyleSheet},
};
use oxvg_ast::{
    atom::Atom,
    attribute::Attr,
    element::Element,
    name::Name,
    visitor::{Context, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

use super::{inline_styles, ContextFlags};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RemoveUnused {
    False,
    True,
    Force,
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
    pub remove_unused: Option<RemoveUnused>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for MinifyStyles {
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
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        self.content(element, context);
        Self::attr(element);
        Ok(())
    }
}

impl MinifyStyles {
    fn content<'arena, E: Element<'arena>>(
        &self,
        element: &E,
        context: &mut Context<'arena, '_, '_, E>,
    ) {
        let name = element.qual_name();
        if name.prefix().is_some() {
            return;
        }

        if name.local_name().as_ref() != "style" {
            return;
        }

        let Some(css) = element.text_content() else {
            return;
        };
        if css.is_empty() {
            return;
        }
        let mut css = match StyleSheet::parse(
            &css,
            ParserOptions {
                flags: ParserFlags::all(),
                ..ParserOptions::default()
            },
        ) {
            Ok(css) => css,
            Err(e) => {
                log::debug!("failed to parse stylesheet: {e}");
                return;
            }
        };
        if let Some(matched_selectors) = self.remove_unused_selectors(&mut css.rules, context) {
            css.rules = matched_selectors;
        }
        let _ = css.minify(MinifyOptions::default());
        let css = match css.to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        }) {
            Ok(css) => css,
            Err(e) => {
                log::debug!("failed to print stylesheet: {e}");
                return;
            }
        };

        if css.code.is_empty() {
            log::debug!("removing element: all styles removed");
            element.remove();
        } else {
            element
                .clone()
                .set_text_content(css.code.into(), &context.info.arena);
        }
    }

    fn remove_unused_selectors<'arena, 'a, E: Element<'arena>>(
        &self,
        css: &mut CssRuleList<'a>,
        context: &Context<'arena, '_, '_, E>,
    ) -> Option<CssRuleList<'a>> {
        let remove_unused = if self.remove_unused != Some(RemoveUnused::Force)
            && context.flags.contains(ContextFlags::has_script_ref)
        {
            Some(RemoveUnused::False)
        } else {
            self.remove_unused
        };
        if remove_unused.unwrap_or(default_remove_unused()) == RemoveUnused::False {
            return None;
        }

        let options = inline_styles::InlineStyles {
            use_mqs: vec!["*".to_string()],
            use_pseudos: vec!["*".to_string()],
            only_matched_once: false,
            ..inline_styles::InlineStyles::default()
        };
        let state = inline_styles::State::new(&options);
        options.take_matching_selectors(css, context, &state)
    }

    fn attr<'arena, E: Element<'arena>>(element: &E) {
        let style_name = "style".into();
        let Some(mut style) = element.get_attribute_node_local_mut(&style_name) else {
            return;
        };
        let value = style.value().as_str();
        let mut css_source = match StyleAttribute::parse(value, ParserOptions::default()) {
            Ok(css) => css,
            Err(e) => {
                log::debug!("failed to parse attribute: {e}");
                return;
            }
        };
        css_source.minify(MinifyOptions::default());
        let css = match css_source.to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        }) {
            Ok(css) => css,
            Err(e) => {
                log::debug!("failed to print style attribute: {e}");
                return;
            }
        };
        let css_atom = css.code.into();
        drop(css_source);

        style.set_value(css_atom);
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
        }
    }
}

const fn default_remove_unused() -> RemoveUnused {
    RemoveUnused::True
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
