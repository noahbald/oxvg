use std::{collections::HashMap, path::PathBuf};

use itertools::Itertools;
use lightningcss::{
    printer::PrinterOptions,
    selector::Component,
    stylesheet::{ParserOptions, StyleSheet},
    traits::ToCss,
    visit_types,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node::{self, Node},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::{collections::REFERENCES_PROPS, regex::REFERENCES_URL};
use regex::{Captures, Match};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg(not(feature = "napi"))]
type Generator = Box<fn(&Option<PrefixGeneratorInfo>) -> String>;

#[cfg(feature = "napi")]
#[derive(Clone, derive_more::Debug)]
pub struct Generator {
    #[debug(skip)]
    pub callback: std::sync::Arc<
        napi::threadsafe_function::ThreadsafeFunction<Option<PrefixGeneratorInfo>, String>,
    >,
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::FromNapiValue for Generator {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        Ok(Self {
            callback: std::sync::Arc::from_napi_value(env, napi_val)?,
        })
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ToNapiValue for Generator {
    unsafe fn to_napi_value(
        _env: napi::sys::napi_env,
        _val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        todo!("converting `Generator` to napi value not yet supported")
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::TypeName for Generator {
    fn type_name() -> &'static str {
        std::sync::Arc::<
            napi::threadsafe_function::ThreadsafeFunction<Option<PrefixGeneratorInfo>, String>,
        >::type_name()
    }

    fn value_type() -> napi::ValueType {
        std::sync::Arc::<
            napi::threadsafe_function::ThreadsafeFunction<Option<PrefixGeneratorInfo>, String>,
        >::value_type()
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ValidateNapiValue for Generator {
    unsafe fn validate(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<napi::sys::napi_value> {
        std::sync::Arc::<
            napi::threadsafe_function::ThreadsafeFunction<Option<PrefixGeneratorInfo>, String>,
        >::validate(env, napi_val)
    }
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Default, Clone, Debug)]
/// Various types of ways prefixes can be generated for an id.
pub enum PrefixGenerator {
    #[cfg(feature = "napi")]
    /// A function to create a dynamic prefix
    Generator(#[napi(ts_type = "(info?: PrefixGeneratorInfo) => string")] Generator),
    #[cfg(not(feature = "napi"))]
    /// A function to create a dynamic prefix
    Generator(Generator),
    /// A string to use as a prefix
    Prefix(String),
    /// No prefix
    None,
    /// Use "prefix" as prefix
    #[default]
    Default,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Debug)]
/// Contextual information about the element that can be used to generate a prefix
pub struct PrefixGeneratorInfo {
    /// The file path of the processed document the element belongs to.
    pub path: Option<String>,
    /// The name of the element.
    pub name: String,
    /// The attributes of the element.
    pub attributes: Vec<(String, String)>,
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

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for PrefixIds {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if !self.prefix_ids && !self.prefix_class_names {
            PrepareOutcome::skip
        } else {
            PrepareOutcome::none
        })
    }

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let mut prefix_generator =
            GeneratePrefix::new(element, context.info, &self.prefix, &self.delim);
        if element.prefix().is_none()
            && element.local_name().as_ref() == "style"
            && self
                .prefix_selectors(element, &mut prefix_generator, context.info)
                .is_none()
        {
            return Ok(());
        }

        for mut attr in element.attributes().into_iter_mut() {
            let value: &str = attr.value().as_ref();
            if value.is_empty() {
                continue;
            }

            let prefix = attr.prefix().as_ref().map(AsRef::as_ref);
            let local_name = attr.local_name().as_ref();

            if prefix.is_none() && local_name == "id" {
                if self.prefix_ids {
                    log::debug!("prefixing id");
                    if let Some(new_id) = Self::prefix_id(value, &mut prefix_generator)? {
                        attr.set_value(new_id.into());
                    }
                }
            } else if prefix.is_none() && local_name == "class" {
                if self.prefix_class_names {
                    log::debug!("prefixing class");
                    let value = value
                        .split_whitespace()
                        .filter_map(|s| Self::prefix_id(s, &mut prefix_generator).unwrap())
                        .join(" ");
                    attr.set_value(value.into());
                }
            } else if prefix.is_none_or(|p| p == "xlink") && local_name == "href" {
                log::debug!("prefixing reference");
                if let Some(new_ref) = Self::prefix_reference(value, &mut prefix_generator)? {
                    attr.set_value(new_ref.into());
                }
            } else if prefix.is_none() && matches!(local_name, "begin" | "end") {
                log::debug!("prefixing animation");
                #[allow(clippy::case_sensitive_file_extension_comparisons)]
                let mut parts = value.split(';').map(str::trim).map(|s| {
                    if s.ends_with(".end") || s.ends_with(".start") {
                        let (id, postfix) =
                            s.split_once('.').expect("should end with `.(end|start)`");
                        if let Some(id) = Self::prefix_id(id, &mut prefix_generator).unwrap() {
                            format!("{id}.{postfix}")
                        } else {
                            s.to_string()
                        }
                    } else {
                        s.to_string()
                    }
                });
                let new_animation_timing = parts.join("; ").into();
                attr.set_value(new_animation_timing);
            } else if prefix.is_none() && REFERENCES_PROPS.contains(local_name) {
                log::debug!("prefixing url");
                let new_value = REFERENCES_URL
                    .replace_all(value, |caps: &Captures| {
                        if let Some(prefix) =
                            Self::prefix_reference(&caps[1], &mut prefix_generator).unwrap()
                        {
                            let start = if caps[0].starts_with(':') { ":" } else { "" };
                            format!("{start}url({prefix})")
                        } else {
                            caps[0].to_string()
                        }
                    })
                    .as_ref()
                    .into();
                attr.set_value(new_value);
            }
        }

        Ok(())
    }
}

impl<'i> lightningcss::visitor::Visitor<'i> for CssVisitor<'_, '_> {
    type Error = String;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        if self.ids {
            visit_types!(SELECTORS | URLS)
        } else {
            visit_types!(SELECTORS)
        }
    }

    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'i>,
    ) -> Result<(), Self::Error> {
        selector.iter_mut_raw_match_order().try_for_each(|c| {
            if matches!(c, Component::Class(_) if !self.class_names)
                || matches!(c, Component::ID(_) if !self.ids)
            {
                return Ok(());
            }
            if let Component::ID(ident) | Component::Class(ident) = c {
                if let Some(new_ident) = PrefixIds::prefix_id(ident, self.generator)? {
                    *ident = new_ident.into();
                }
            }
            Ok(())
        })
    }

    fn visit_url(
        &mut self,
        url: &mut lightningcss::values::url::Url<'i>,
    ) -> Result<(), Self::Error> {
        if let Some(new_url) = PrefixIds::prefix_reference(&url.url, self.generator)? {
            url.url = new_url.into();
        }
        Ok(())
    }
}

impl PrefixIds {
    fn prefix_selectors<'arena, E: Element<'arena>>(
        &self,
        element: &mut E,
        prefix_generator: &mut GeneratePrefix,
        info: &Info<'arena, E>,
    ) -> Option<()> {
        if element.is_empty() {
            return None;
        }

        log::debug!("prefixing selectors for style element");
        element.child_nodes_iter().for_each(|child| {
            if !matches!(
                child.node_type(),
                node::Type::Text | node::Type::CDataSection
            ) {
                return;
            }

            let Some(css) = child.text_content() else {
                return;
            };
            let Ok(mut css_ast) = StyleSheet::parse(&css, ParserOptions::default()) else {
                return;
            };
            self.prefix_styles(&mut css_ast, prefix_generator);

            let options = PrinterOptions {
                minify: true,
                ..PrinterOptions::default()
            };
            let Ok(css) = css_ast.rules.to_css_string(options) else {
                return;
            };
            child.set_text_content(css.into(), &info.arena);
        });
        Some(())
    }

    fn prefix_styles(&self, css: &mut StyleSheet, prefix_generator: &mut GeneratePrefix) {
        use lightningcss::visitor::Visitor;

        let mut visitor = CssVisitor {
            generator: prefix_generator,
            ids: self.prefix_ids,
            class_names: self.prefix_class_names,
        };
        let _ = visitor.visit_stylesheet(css);
    }

    fn prefix_id(
        ident: &str,
        prefix_generator: &mut GeneratePrefix,
    ) -> Result<Option<String>, String> {
        let prefix = prefix_generator.generate(ident)?;
        if ident.starts_with(&prefix) {
            return Ok(None);
        }
        Ok(Some(format!("{prefix}{ident}")))
    }

    fn prefix_reference(
        url: &str,
        prefix_generator: &mut GeneratePrefix,
    ) -> Result<Option<String>, String> {
        let reference = url.strip_prefix('#').unwrap_or(url);
        let prefix = prefix_generator.generate(reference)?;
        if reference.starts_with(&prefix) {
            return Ok(None);
        }
        Ok(Some(format!("#{prefix}{reference}")))
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
            PrefixGenerator::Generator(..) => {
                log::warn!("Cannot serialize PrefixGenerator function");
                false.serialize(serializer)
            }
        }
    }
}

#[derive(Debug)]
struct GeneratePrefix<'a> {
    info: Option<PrefixGeneratorInfo>,
    prefix_generator: &'a PrefixGenerator,
    delim: &'a str,
    path: &'a Option<PathBuf>,
    history: HashMap<String, String>,
}

impl<'a> GeneratePrefix<'a> {
    fn new<'arena, E: Element<'arena>>(
        element: &E,
        info: &'a Info<'arena, E>,
        prefix_generator: &'a PrefixGenerator,
        delim: &'a str,
    ) -> Self {
        let path = &info.path;
        let info = match prefix_generator {
            PrefixGenerator::Generator(_) => Some(PrefixGeneratorInfo {
                path: info.path.as_ref().map(|p| p.to_string_lossy().to_string()),
                name: element.qual_name().formatter().to_string(),
                attributes: element
                    .attributes()
                    .into_iter()
                    .map(|a| (a.name().formatter().to_string(), a.value().to_string()))
                    .collect(),
            }),
            _ => None,
        };
        Self {
            info,
            prefix_generator,
            delim,
            path,
            history: HashMap::new(),
        }
    }

    fn generate(&mut self, body: &str) -> Result<String, String> {
        Ok(match self.prefix_generator {
            PrefixGenerator::Generator(f) => {
                if let Some(prefix) = self.history.get(body) {
                    return Ok((*prefix).to_string());
                }
                #[cfg(not(feature = "napi"))]
                let prefix = f(&self.info);
                #[cfg(feature = "napi")]
                let prefix = {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let t = std::thread::spawn({
                        let info = self.info.clone();
                        let f = f.clone();
                        move || {
                            f.callback.call_with_return_value(
                                Ok(info),
                                napi::threadsafe_function::ThreadsafeFunctionCallMode::Blocking,
                                move |result, _| match result {
                                    Ok(s) => {
                                        tx.send(Ok(s)).map_err(|e| {
                                            napi::Error::new(napi::Status::GenericFailure, e)
                                        })?;
                                        Ok(())
                                    }
                                    Err(err) => {
                                        tx.send(Err(err.to_string())).map_err(|e| {
                                            napi::Error::new(napi::Status::GenericFailure, e)
                                        })?;
                                        Err(err)
                                    }
                                },
                            )
                        }
                    });
                    let prefix = rx
                        .recv_timeout(std::time::Duration::new(5, 0))
                        .map_err(|err| err.to_string());
                    let status = t.join().map_err(|_| String::from("thread failed"))?;
                    if status == napi::Status::Ok {
                        prefix??
                    } else {
                        return Err(status.to_string());
                    }
                };

                self.history.insert(body.to_string(), prefix.clone());
                prefix
            }
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
        })
    }
}

fn get_basename(path: &std::path::Path) -> Option<Match> {
    let path = path.as_os_str().to_str()?;
    BASENAME_CAPTURE
        .captures_iter(path)
        .next()
        .and_then(|m| m.get(0))
}

lazy_static! {
    static ref ESCAPE_IDENTIFIER_NAME: regex::Regex = regex::Regex::new("[. ]").unwrap();
    static ref BASENAME_CAPTURE: regex::Regex = regex::Regex::new(r"[/\\]?([^/\\]+)").unwrap();
}

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
    #a {}
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
