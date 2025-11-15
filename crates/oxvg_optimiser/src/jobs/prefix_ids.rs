use std::{path::PathBuf, sync::LazyLock};

use lightningcss::{
    rules::CssRuleList, selector::Component, values::ident::Ident, visit_types, visitor::Visit,
};
use oxvg_ast::{
    element::Element,
    is_element,
    visitor::{Context, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::content_type::Reference;
use regex::Match;
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "napi", napi)]
#[derive(Default, Clone, Debug)]
/// Various types of ways prefixes can be generated for an id.
pub enum PrefixGenerator {
    /// A string to use as a prefix
    Prefix(String),
    /// No prefix
    None,
    /// Use "prefix" as prefix
    #[default]
    Default,
}

fn default_delim() -> String {
    "__".to_string()
}

const fn default_prefix_ids() -> bool {
    true
}

const fn default_prefix_class_names() -> bool {
    true
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
/// Prefix element ids and classnames with the filename or provided string. This
/// is useful for reducing the likelihood of conflicts when inlining SVGs.
///
/// See [`super::CleanupIds`] for more details.
///
/// # Differences to SVGO
///
/// Custom generator functions are not supported.
///
/// # Correctness
///
/// Prefixing ids on inlined SVGs may affect scripting and CSS.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct PrefixIds {
    #[serde(default = "default_delim")]
    /// Content to insert between the prefix and original value.
    pub delim: String,
    #[serde(default)]
    /// A string or generator that resolves to a string
    #[cfg_attr(feature = "wasm", tsify(type = "string | boolean | null"))]
    pub prefix: PrefixGenerator,
    #[serde(default = "default_prefix_ids")]
    /// Whether to prefix ids
    pub prefix_ids: bool,
    #[serde(default = "default_prefix_class_names")]
    /// Whether to prefix classnames
    pub prefix_class_names: bool,
}

impl Default for PrefixIds {
    fn default() -> Self {
        PrefixIds {
            delim: default_delim(),
            prefix: PrefixGenerator::default(),
            prefix_ids: default_prefix_ids(),
            prefix_class_names: default_prefix_class_names(),
        }
    }
}

struct CssVisitor<'a, 'b> {
    generator: &'a mut GeneratePrefix<'b>,
    ids: bool,
    class_names: bool,
}

impl<'input, 'arena> Visitor<'input, 'arena> for PrefixIds {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if !self.prefix_ids && !self.prefix_class_names {
            PrepareOutcome::skip
        } else {
            PrepareOutcome::none
        })
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut prefix_generator = GeneratePrefix::new(context.info, &self.prefix, &self.delim);
        if is_element!(element, Style)
            && self
                .prefix_selectors(element, &mut prefix_generator)
                .is_none()
        {
            return Ok(());
        }

        for mut attr in element.attributes().into_iter_mut() {
            let mut value = attr.value_mut();
            if self.prefix_ids {
                log::debug!("prefixing id");
                value.visit_id(|id| {
                    Self::prefix_id(id, &mut prefix_generator);
                });
            }
            if self.prefix_class_names {
                log::debug!("prefixing class");
                value.visit_class(|class| {
                    Self::prefix_id(class, &mut prefix_generator);
                });
            }
            value.visit_url(|url| {
                Self::prefix_reference(url, &mut prefix_generator);
            });
        }

        Ok(())
    }
}

impl<'input> lightningcss::visitor::Visitor<'input> for CssVisitor<'_, '_> {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        if self.ids {
            visit_types!(SELECTORS | URLS)
        } else {
            visit_types!(SELECTORS)
        }
    }

    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'input>,
    ) -> Result<(), Self::Error> {
        selector.iter_mut_raw_match_order().try_for_each(|c| {
            if matches!(c, Component::Class(_) if !self.class_names)
                || matches!(c, Component::ID(_) if !self.ids)
            {
                return Ok(());
            }
            if let Component::ID(Ident(ident)) | Component::Class(Ident(ident)) = c {
                PrefixIds::prefix_id(Reference::Css(ident), self.generator);
            }
            Ok(())
        })
    }

    fn visit_url(
        &mut self,
        url: &mut lightningcss::values::url::Url<'input>,
    ) -> Result<(), Self::Error> {
        PrefixIds::prefix_reference(Reference::Css(&mut url.url), self.generator);
        Ok(())
    }
}

impl PrefixIds {
    fn prefix_selectors(
        &self,
        element: &Element,
        prefix_generator: &mut GeneratePrefix,
    ) -> Option<()> {
        if element.is_empty() {
            return None;
        }

        log::debug!("prefixing selectors for style element");
        element.child_nodes_iter().for_each(|child| {
            let Some(css) = child.style() else {
                return;
            };
            let mut css = css.borrow_mut();
            self.prefix_styles(&mut css, prefix_generator);
        });
        Some(())
    }

    fn prefix_styles(&self, css: &mut CssRuleList, prefix_generator: &mut GeneratePrefix) {
        let mut visitor = CssVisitor {
            generator: prefix_generator,
            ids: self.prefix_ids,
            class_names: self.prefix_class_names,
        };
        css.visit(&mut visitor).ok();
    }

    fn prefix_id(ident: Reference, prefix_generator: &mut GeneratePrefix) {
        let prefix = prefix_generator.generate();
        if ident.starts_with(&prefix) {
            return;
        }
        let new_ident = format!("{prefix}{}", &*ident);
        match ident {
            Reference::Atom(atom) => *atom = new_ident.into(),
            Reference::Css(css) => *css = new_ident.into(),
        }
    }

    fn prefix_reference(url: Reference, prefix_generator: &mut GeneratePrefix) {
        if !url.starts_with('#') {
            return;
        }
        let reference = url.strip_prefix('#').unwrap();
        let prefix = prefix_generator.generate();
        if reference.starts_with(&prefix) {
            return;
        }
        let new_url = format!("#{prefix}{reference}");
        match url {
            Reference::Atom(atom) => *atom = new_url.into(),
            Reference::Css(cow) => *cow = new_url.into(),
        }
    }
}

impl<'de> Deserialize<'de> for PrefixGenerator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        match value {
            serde_json::Value::String(string) => Ok(PrefixGenerator::Prefix(string)),
            serde_json::Value::Bool(bool) => {
                if bool {
                    Ok(PrefixGenerator::Default)
                } else {
                    Ok(PrefixGenerator::None)
                }
            }
            serde_json::Value::Null => Ok(PrefixGenerator::Default),
            _ => Err(serde::de::Error::custom(
                "expected a string, boolean, or null",
            )),
        }
    }
}

impl Serialize for PrefixGenerator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PrefixGenerator::Prefix(prefix) => prefix.serialize(serializer),
            PrefixGenerator::Default => true.serialize(serializer),
            PrefixGenerator::None => false.serialize(serializer),
        }
    }
}

#[derive(Debug)]
struct GeneratePrefix<'a> {
    prefix_generator: &'a PrefixGenerator,
    delim: &'a str,
    path: &'a Option<PathBuf>,
}

impl<'a> GeneratePrefix<'a> {
    fn new<'input, 'arena>(
        info: &'a Info<'input, 'arena>,
        prefix_generator: &'a PrefixGenerator,
        delim: &'a str,
    ) -> Self {
        let path = &info.path;
        Self {
            prefix_generator,
            delim,
            path,
        }
    }

    fn generate(&mut self) -> String {
        match self.prefix_generator {
            PrefixGenerator::Prefix(s) => format!("{s}{}", self.delim),
            PrefixGenerator::None => String::new(),
            PrefixGenerator::Default => match &self.path {
                Some(path) => match get_basename(path) {
                    Some(name) => format!(
                        "{}{}",
                        ESCAPE_IDENTIFIER_NAME.replace(name.as_str(), "_"),
                        self.delim
                    ),
                    None => self.delim.to_string(),
                },
                None => format!("prefix{}", self.delim),
            },
        }
    }
}

fn get_basename(path: &std::path::Path) -> Option<Match> {
    let path = path.as_os_str().to_str()?;
    BASENAME_CAPTURE
        .captures_iter(path)
        .next()
        .and_then(|m| m.get(0))
}

static ESCAPE_IDENTIFIER_NAME: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("[. ]").unwrap());
static BASENAME_CAPTURE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"[/\\]?([^/\\]+)").unwrap());

#[test]
#[allow(clippy::too_many_lines)]
fn prefix_ids() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds"
        } }"#,
        Some(
            r#"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <!-- update selectors and attributes for classes and ids -->
    <style>
        .test {
            color: blue;
        }
        #test {
            color: red;
        }

    </style>
    <rect class="test" x="10" y="10" width="100" height="100"/>
    <rect class="" id="test" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_02_svg_txt"
        } }"#,
        Some(
            r#"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <!-- prefix attribute url -->
    <defs>
        <linearGradient id="MyGradient">
            <stop offset="5%" stop-color="green"/>
            <stop offset="95%" stop-color="gold"/>
        </linearGradient>
    </defs>
    <rect fill="url(#MyGradient)" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_03_svg_txt"
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- add prefix to xlink:href -->
    <use xlink:href="#Port"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_04_svg_txt"
        } }"#,
        Some(
            r##"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <!-- add prefix to css urls -->
    <style>
        rect {
            cursor: pointer;
            shape-rendering: crispEdges;
            fill:url("#MyGradient");
        }

    </style>
    <rect x="10" y="10" width="100" height="100"/>
    <rect x="10" y="10" width="100" height="100"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_05_svg_txt"
        } }"#,
        Some(
            r#"<svg width="340" height="120" xmlns="http://www.w3.org/2000/svg">
    <defs>
        <linearGradient id="gradient_1">
            <stop offset="5%" stop-color="green"/>
            <stop offset="95%" stop-color="gold"/>
        </linearGradient>
        <linearGradient id="gradient_2">
            <stop offset="5%" stop-color="red"/>
            <stop offset="95%" stop-color="black"/>
        </linearGradient>
        <linearGradient id="gradient_3">
            <stop offset="5%" stop-color="blue"/>
            <stop offset="95%" stop-color="orange"/>
        </linearGradient>
    </defs>
    <rect fill="url(#gradient_1)" x="10" y="10" width="100" height="100"/>
    <rect fill="url(#gradient_2)" x="120" y="10" width="100" height="100"/>
    <rect fill="url(#gradient_3)" x="230" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_06_svg_txt"
        } }"#,
        Some(
            r#"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <!-- Prefix multiple idents per attr/selector -->
    <style>
        .test {
            color: blue;
        }
        .test2 {
            color: green;
        }
        #test {
            color: red;
        }
        .test3 .test4 {
            color: black;
        }
        .test5.test6 {
            color: brown;
        }
        .test5.test6 #test7 {
            color: yellow;
        }
    </style>
    <rect class="test" x="10" y="10" width="100" height="100"/>
    <rect class="test test2" x="10" y="10" width="100" height="100"/>
    <rect class="test  test2" x="10" y="10" width="100" height="100"/>
    <rect class="" id="test" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_07_svg_txt",
            "prefixIds": false
        } }"#,
        Some(
            r#"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <style>
        .test {
            color: blue;
        }
        #test {
            color: red;
        }

    </style>
    <rect class="test" x="10" y="10" width="100" height="100"/>
    <rect class="" id="test" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_08_svg_txt",
            "prefixClassNames": false
        } }"#,
        Some(
            r#"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <style>
        .test {
            color: blue;
        }
        #test {
            color: red;
        }

    </style>
    <rect class="test" x="10" y="10" width="100" height="100"/>
    <rect class="" id="test" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_09_svg_txt",
            "prefixIds": false,
            "prefixClassNames": false
        } }"#,
        Some(
            r#"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <style>
        .test {
            color: blue;
        }
        #test {
            color: red;
        }

    </style>
    <rect class="test" x="10" y="10" width="100" height="100"/>
    <rect class="" id="test" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_10_svg_txt"
        } }"#,
        Some(
            r#"<g xmlns="http://www.w3.org/2000/svg" transform="translate(130, 112)">
    <path class="st1" d="M27,0h-37v64C-10,64,27,64.2,27,0z" transform="scale(0.811377 1)">
    <animateTransform id="t_1s" attributeName="transform" type="scale" from="1 1" to="-1 1" begin="0s; t_2s.end" dur="0.5s" repeatCount="0"/>
    <animateTransform id="t_2s" attributeName="transform" type="scale" from="-1 1" to="1 1" begin="t_1s.end" dur="0.5s" repeatCount="0"/>
    </path>
</g>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_11_svg_txt"
        } }"#,
        Some(
            r#"<svg width="120" height="120" xmlns="http://www.w3.org/2000/svg">
    <defs>
        <linearGradient id="fill"/>
        <linearGradient id="stroke"/>
    </defs>
    <rect style="fill:url(#fill); stroke: url(#stroke)" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_12_svg_txt"
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1">
  <style>
    <!-- uwu -->
    #a { color: red; }
  </style>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "prefixIds": {
            "prefix": "prefixIds_13_svg_txt"
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1">
  <style>
    <!-- uwu -->
    #a13 {} <!-- xyz -->
    #b13 {}
  </style>
</svg>"#
        )
    )?);

    Ok(())
}
