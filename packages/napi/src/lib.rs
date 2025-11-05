//! NAPI bindings for OXVG
use napi::{Error, Status};
use oxvg_ast::{
  arena::Allocator,
  parse::roxmltree::parse,
  serialize::{self, Node as _, Options},
  visitor::Info,
};
use oxvg_optimiser::{Extends, Jobs};
#[macro_use]
extern crate napi_derive;

#[napi]
#[allow(clippy::needless_pass_by_value)]
/// Optimise an SVG document using the provided config
///
/// # Errors
/// - If the document fails to parse
/// - If any of the optimisations fail
/// - If the optimised document fails to serialize
///
/// # Examples
///
/// Optimise svg with the default configuration
///
/// ```js
/// import { optimise } from "@oxvg/napi";
///
/// const result = optimise(`<svg />`);
/// ```
///
/// Or, provide your own config
///
/// ```js
/// import { optimise } from "@oxvg/napi";
///
/// // Only optimise path data
/// const result = optimise(`<svg />`, { convertPathData: {} });
/// ```
///
/// Or, extend a preset
///
/// ```js
/// import { optimise, extend, Extends } from "@oxvg/napi";
///
/// const result = optimise(
///     `<svg />`,
///     extend(Extends.Default, { convertPathData: { removeUseless: false } }),
/// );
/// ```
pub fn optimise(svg: String, config: Option<Jobs>) -> Result<String, Error<Status>> {
  let xml = roxmltree::Document::parse(&svg).map_err(generic_error)?;
  let values = Allocator::new_values();
  let mut arena = Allocator::new_arena();
  let mut allocator = Allocator::new(&mut arena, &values);
  let dom =
    parse(&xml, &mut allocator).map_err(|e| Error::new(Status::InvalidArg, e.to_string()))?;
  config
    .unwrap_or_default()
    .run(dom, &Info::new(allocator))
    .map_err(generic_error)?;

  dom
    .serialize_with_options(Options {
      indent: serialize::Indent::None,
      ..Default::default()
    })
    .map_err(generic_error)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
/// Returns the given config with omitted options replaced with the config provided by `extends`.
/// I.e. acts like `{ ...extends, ...config }`
pub fn extend(extend: Extends, config: Option<Jobs>) -> Jobs {
  match config {
    Some(ref jobs) => extend.extend(jobs),
    None => extend.jobs(),
  }
}

#[napi]
/// Converts a JSON value of SVGO's `Config["plugins"]` into [`Jobs`].
///
/// Note that this will deduplicate any plugins listed.
///
/// # Errors
///
/// If a config file cannot be deserialized into jobs. This may fail even if
/// the config is valid for SVGO, such as if
///
/// - The config contains custom plugins
/// - The plugin parameters are incompatible with OXVG
/// - The underlying deserialization process fails
///
/// If you believe an errors should be fixed, please raise an issue
/// [here](https://github.com/noahbald/oxvg/issues)
pub fn convert_svgo_config(config: Option<Vec<serde_json::Value>>) -> Result<Jobs, Error<Status>> {
  Jobs::from_svgo_plugin_config(config).map_err(generic_error)
}

#[allow(clippy::needless_pass_by_value)]
fn generic_error<T: ToString>(err: T) -> Error<Status> {
  Error::new(Status::GenericFailure, err.to_string())
}
