//! NAPI bindings for OXVG
use std::{collections::HashMap, io::Write as _, marker::PhantomData};

use oxvg_jsx::{
  config::{
    ExpandProps, ExportType, Icon, JSXRuntime, JsxRuntimeImport, State, Template, TemplateAST,
    TemplateContext, VariablesString,
  },
  error::TemplateError,
  Config,
};
use oxvg_optimiser::Jobs;

use napi::{
  bindgen_prelude::{FnArgs, Function},
  Status,
};

#[macro_use]
extern crate napi_derive;

#[napi(object)]
/// Configures what changes are made when transforming an SVG document to JSX.
pub struct JSXConfig<'env> {
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
  /// Whether to use `oxvg_optimiser` to optimise SVG document before processing it.
  pub oxvg: Option<bool>,
  /// Whether to use `oxvg_optimiser` to optimise SVG document before processing it.
  pub svgo: Option<bool>,
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
  /// A function used to build a custom component.
  pub template: Option<Function<'env, FnArgs<(VariablesString, TemplateContext)>, String>>,
  /// Whether to emit warnings.
  pub warn: Option<bool>,
}

impl From<&JSXConfig<'_>> for Config {
  fn from(val: &JSXConfig<'_>) -> Self {
    Config {
      r#ref: val.r#ref,
      title_prop: val.title_prop,
      desc_prop: val.desc_prop,
      expand_props: val.expand_props,
      dimensions: val.dimensions,
      icon: val.icon.clone(),
      native: val.native,
      svg_props: val.svg_props.clone(),
      replace_attr_values: val.replace_attr_values.clone(),
      typescript: val.typescript,
      oxvg: val.oxvg.or(val.svgo),
      oxvg_config: val.oxvg_config.clone(),
      memo: val.memo,
      export_type: val.export_type,
      named_export: val.named_export.clone(),
      jsx_runtime: val.jsx_runtime,
      jsx_runtime_import: val.jsx_runtime_import.clone(),
      warn: val.warn,
    }
  }
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
/// Transform an SVG document into a JSX component.
///
/// # Errors
/// - If the document fails to build
/// - If there is an error parsing the configuration
/// - If any of the optimisations fail
///
/// # Examples
///
/// Transform an svg with the default configuration
///
/// ```js
/// import { transform } from "@oxvg/jsx";
///
/// const result = transform(`<svg />`);
/// ```
///
/// Or, provide your own config
///
/// ```js
/// import { transform } from "@oxvg/jsx";
///
/// const result = optimise(`<svg />`, { icon: true });
/// ```
///
/// Or, include a custom template
///
/// ```js
/// import { transform } from "@oxvg/jsx";
///
/// const result = optimise(
///     `<svg />`,
///     { template: ({ imports, interfaces, componentName, props, jsx, exports }) => `${imports}
/// import PropTypes from 'prop-types';
/// ${interfaces}
///
/// function ${componentName}(${props}) {
///   return ${jsx};
/// }
///
/// ${componentName}.propTypes = {
///   title: PropTypes.string,
/// };
///
/// ${exports}
/// `}
/// );
/// ```
pub fn transform(
  code: String,
  config: Option<JSXConfig>,
  state: Option<State>,
) -> napi::Result<String> {
  let base_config = config.as_ref().map(Into::into);
  let template = config.and_then(|c| c.template).map(|f| {
    Template::<Vec<u8>, TemplateAST<_>, _>::String(
      move |w, vars, ctx| -> Result<(), TemplateError> {
        let string = f.call((vars, ctx).into()).map_err(|_| TemplateError)?;
        w.write_all(string.as_bytes()).map_err(|_| TemplateError)
      },
      PhantomData,
    )
  });

  swc_core::common::GLOBALS
    .set(&swc_core::common::Globals::new(), || {
      oxvg_ast::parse::roxmltree::parse(&code, move |root, allocator| {
        let mut buf = Vec::new();
        oxvg_jsx::transform(root, allocator, base_config, state, template, &mut buf)
          .map_err(|err| err.to_string())?;
        String::from_utf8(buf).map_err(|err| err.to_string())
      })
    })
    .map_err(|err| napi::Error::new(Status::GenericFailure, err.to_string()))?
    .map_err(|err| napi::Error::new(Status::GenericFailure, err))
}
