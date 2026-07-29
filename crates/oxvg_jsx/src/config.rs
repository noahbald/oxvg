//! Config types and Template Option types.
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::marker::PhantomData;

#[cfg(feature = "optimise")]
use oxvg_optimiser::Jobs;

#[cfg(feature = "swc_core")]
use crate::error::TemplateError;

#[derive(Default, Clone)]
#[cfg_attr(feature = "napi", napi(object))]
/// Configures what changes are made when transforming an SVG document to JSX.
pub struct Config {
    /// Whether to add `ref` attribute with `forwardRef`.
    pub r#ref: Option<bool>,
    /// Whether to add `title` and `titleId` prop to pass to `<title>` element.
    pub title_prop: Option<bool>,
    /// Whether to add `desc` and `descId` prop to pass to `<desc>` element.
    pub desc_prop: Option<bool>,
    /// Whether to pass `...props` to `<svg>` element.
    pub expand_props: Option<ExpandProps>,
    /// Whether to strip `width` and `height` attributes from `<svg>` element.
    pub dimensions: Option<bool>,
    /// Whether to set `width` and `height` to specified icon type.
    pub icon: Option<Icon>,
    /// Whether to use React-Native elements.
    pub native: Option<bool>,
    /// A set of attributes to apply to `<svg>` element. Values given as `"{...}"` will
    /// be parsed as JSX expressions instead of strings.
    pub svg_props: Option<HashMap<String, String>>,
    /// A set of values to replace the original values specified in the keys. Values given as `"{...}"` will
    /// be parsed as JSX expressions instead of strings.
    ///
    /// A value of `"#000": "currentColor"` will replace all `#000` values with `"currentColor"`.
    pub replace_attr_values: Option<HashMap<String, String>>,
    /// Whether to generate TypeScript types.
    pub typescript: Option<bool>,
    #[cfg(feature = "optimise")]
    /// Whether to use `oxvg_optimiser` to optimise SVG document before processing it.
    pub oxvg: Option<bool>,
    #[cfg(feature = "optimise")]
    /// `oxvg_optimiser` config to apply to SVG document.
    pub oxvg_config: Option<Jobs>,
    /// Whether to use `React.memo` on the exported component.
    pub memo: Option<bool>,
    /// Whether to use `default` keyword when exporting component.
    pub export_type: Option<ExportType>,
    /// An alias for the export when using a named export-type.
    pub named_export: Option<String>,
    /// When given, controls where to import JSX runtime.
    pub jsx_runtime: Option<JSXRuntime>,
    /// When given, specifies custom JSX runtime.
    pub jsx_runtime_import: Option<JsxRuntimeImport>,
    /// Whether to emit warnings.
    pub warn: Option<bool>,
}

impl Config {
    /// Returns the config as context-options to pass to template function.
    pub fn opts(
        self,
        state: Option<State>,
        native_idents: HashSet<&'static str>,
    ) -> TemplateContextOptions {
        TemplateContextOptions {
            typescript: self.typescript,
            title_prop: self.title_prop,
            desc_prop: self.desc_prop,
            expand_props: self.expand_props,
            r#ref: self.r#ref,
            native: self.native,
            memo: self.memo,
            export_type: self.export_type,
            named_export: self.named_export,
            jsx_runtime: self.jsx_runtime,
            jsx_runtime_import: self.jsx_runtime_import,
            state: state.unwrap_or_default(),
            native_idents: native_idents.into_iter().map(String::from).collect(),
        }
    }

    pub(crate) fn r#ref(&self) -> bool {
        self.r#ref.unwrap_or(false)
    }

    pub(crate) fn title_prop(&self) -> bool {
        self.title_prop.unwrap_or(false)
    }

    pub(crate) fn desc_prop(&self) -> bool {
        self.desc_prop.unwrap_or(false)
    }

    pub(crate) fn expand_props(&self) -> ExpandProps {
        self.expand_props.unwrap_or_default()
    }

    pub(crate) fn dimensions(&self) -> bool {
        self.dimensions.unwrap_or(true)
    }

    pub(crate) fn native(&self) -> bool {
        self.native.unwrap_or(false)
    }

    pub(crate) fn warn(&self) -> bool {
        self.warn.unwrap_or(true)
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "napi", napi)]
/// Specifies which end of an element's attributes to spread into.
pub enum ExpandProps {
    /// `<svg {...props} viewBox="0 0 10 10">`
    Start,
    #[default]
    /// `<svg viewBox="0 0 10 10" {...props}>`
    End,
    /// `<svg viewBox="0 0 10 10">`
    None,
}

impl ExpandProps {
    pub(crate) fn is_some(self) -> bool {
        !self.is_none()
    }

    pub(crate) fn is_none(self) -> bool {
        matches!(self, Self::None)
    }
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
/// Specifies what value to set `width` and `height` on the `<svg>` element.
pub enum Icon {
    /// When `true`, sets `width` and `height` to `"1em"`.
    Bool(bool),
    /// Sets `width` and `height` to the given string.
    String(String),
    /// Sets `width` and `height` to the given number.
    Number(f64),
}

impl Default for Icon {
    fn default() -> Self {
        Icon::Bool(false)
    }
}

/// A template function to write the resulting document after processing is completed.
#[derive(Clone)]
pub enum Template<
    W: Write,
    FA: FnOnce(&mut W, VariablesAST, TemplateContext) -> Result<(), TemplateError> = TemplateAST<W>,
    FS: FnOnce(&mut W, VariablesString, TemplateContext) -> Result<(), TemplateError> = TemplateString<W>
> {
    /// Write AST chunks to the writer.
    AST(FA, PhantomData<W>),
    /// Write string chunks to the writer.
    String(FS, PhantomData<W>),
}

impl<W: Write> Default for Template<W> {
    fn default() -> Self {
        Self::AST(crate::swc::default_template, PhantomData)
    }
}

#[cfg(feature = "swc_core")]
/// A function to join the variables (i.e. the JSX chunks) into a JSX document.
pub type TemplateAST<W> =
    fn(w: &mut W, variables: VariablesAST, context: TemplateContext) -> Result<(), TemplateError>;

#[cfg(feature = "swc_core")]
/// A set of JSX parts to be passed to a template function.
pub type VariablesAST = crate::swc::Variables;

/// A function to join the variables (i.e. the JSX chunks) into a JSX document.
pub type TemplateString<W> = fn(
    w: &mut W,
    variables: VariablesString,
    context: TemplateContext,
) -> Result<(), TemplateError>;

#[cfg_attr(feature = "napi", napi(object))]
/// A set of JSX parts to be passed to a template function.
pub struct VariablesString {
    /// The identifier of the component function name.
    pub component_name: String,
    /// The TypeScript interface declaration for the component's props.
    pub interfaces: String,
    /// The list of parameters of the component function.
    pub props: String,
    /// The list of imports for the start of the module.
    pub imports: String,
    /// The list of exports for the end of the module.
    pub exports: String,
    /// The jsx to be returned by the component.
    pub jsx: String,
}

#[cfg_attr(feature = "napi", napi(object))]
/// Context for the document source passed to the template function.
pub struct TemplateContext {
    /// Options passed to the processor.
    pub options: TemplateContextOptions,
}

#[cfg_attr(feature = "napi", napi(object))]
/// A subset of the [`Config`] passed to the transformer.
pub struct TemplateContextOptions {
    /// Whether to add `ref` attribute with `forwardRef`.
    pub r#ref: Option<bool>,
    /// Whether to add `title` and `titleId` prop to pass to `<title>` element.
    pub title_prop: Option<bool>,
    /// Whether to add `desc` and `descId` prop to pass to `<desc>` element.
    pub desc_prop: Option<bool>,
    /// Whether to pass `...props` to `<svg>` element.
    pub expand_props: Option<ExpandProps>,
    /// Whether to use React-Native elements.
    pub native: Option<bool>,
    /// Whether to generate TypeScript types.
    pub typescript: Option<bool>,
    /// Whether to use `React.memo` on the exported component.
    pub memo: Option<bool>,
    /// Whether to use `default` keyword when exporting component.
    pub export_type: Option<ExportType>,
    /// An alias for the export when using a named export-type.
    pub named_export: Option<String>,
    /// When given, controls where to import JSX runtime.
    pub jsx_runtime: Option<JSXRuntime>,
    /// When given, specifies custom JSX runtime.
    pub jsx_runtime_import: Option<JsxRuntimeImport>,
    /// The state derived from reading the source document.
    pub state: State,
    /// If `native` is given, the list of components to import from react-native.
    pub native_idents: HashSet<String>,
}

impl TemplateContextOptions {
    pub(crate) fn r#ref(&self) -> bool {
        self.r#ref.unwrap_or(false)
    }

    pub(crate) fn title_prop(&self) -> bool {
        self.title_prop.unwrap_or(false)
    }

    pub(crate) fn desc_prop(&self) -> bool {
        self.desc_prop.unwrap_or(false)
    }

    pub(crate) fn expand_props(&self) -> ExpandProps {
        self.expand_props.unwrap_or_default()
    }

    pub(crate) fn native(&self) -> bool {
        self.native.unwrap_or(false)
    }

    pub(crate) fn memo(&self) -> bool {
        self.memo.unwrap_or(false)
    }

    pub(crate) fn typescript(&self) -> bool {
        self.typescript.unwrap_or(false)
    }

    pub(crate) fn export_type(&self) -> ExportType {
        self.export_type.unwrap_or_default()
    }

    pub(crate) fn jsx_runtime(&self) -> JSXRuntime {
        self.jsx_runtime.unwrap_or(JSXRuntime::Classic)
    }

    pub(crate) fn jsx_runtime_import(&self) -> JsxRuntimeImport {
        if let Some(v) = &self.jsx_runtime_import {
            return v.clone();
        }
        match self.jsx_runtime() {
            JSXRuntime::Classic => JsxRuntimeImport {
                source_namespace: None,
                source: "react".into(),
                specifiers: None,
                namespace: Some("React".into()),
                default_specifier: None,
            },
            JSXRuntime::ClassicPreact => JsxRuntimeImport {
                source_namespace: Some("preact".into()),
                source: "preact/compat".into(),
                specifiers: Some(vec!["h".into()]),
                namespace: None,
                default_specifier: None,
            },
            JSXRuntime::Automatic => JsxRuntimeImport {
                source_namespace: None,
                source: "react".into(),
                specifiers: None,
                namespace: None,
                default_specifier: None,
            },
        }
    }
}

#[cfg_attr(feature = "napi", napi(object))]
/// The state derived from reading the source document.
#[derive(Clone)]
pub struct State {
    /// The component name based on the source file-name. `"SvgComponent"` if
    /// the file-name is unknown.
    pub component_name: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            component_name: "SvgComponent".into()
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "napi", napi)]
/// Whether to use `default` keyword when exporting component.
pub enum ExportType {
    /// `export { Name }`
    Named,
    #[default]
    /// `export default Name`
    Default,
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "napi", napi)]
/// When given, controls where to import JSX runtime.
pub enum JSXRuntime {
    #[default]
    /// `import React from "react"`
    Classic,
    /// Omits JSX runtime imports.
    Automatic,
    /// `import { h } from "preact"`
    ClassicPreact,
}

#[derive(Clone)]
#[cfg_attr(feature = "napi", napi(object))]
/// When given, specifies custom JSX runtime.
pub struct JsxRuntimeImport {
    /// `import * as React from "<source_namespace>"`, uses `source` when omitted.
    ///
    /// This is used for classic-style runtimes.
    pub source_namespace: Option<String>,
    /// `import { forwardRef } from "<source>"`
    ///
    /// This is used for TypeScript, `forwardRef`, and `memo` imports.
    pub source: String,
    /// `import { <specifiers> } from "react"`
    ///
    /// This will be applied to the import from `source`.
    pub specifiers: Option<Vec<String>>,
    /// `import * as <namespace> from "react"`.
    ///
    /// This is used for classic-style runtimes.
    pub namespace: Option<String>,
    /// `import  <default_specifiers>  from "react"`
    pub default_specifier: Option<String>,
}
