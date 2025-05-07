//! NAPI bindings for OXVG
use napi::{Error, Status};
use oxvg_ast::{
  implementations::{roxmltree::parse, shared::Element},
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
/// import { optimise } from "@oxvg/wasm";
///
/// const result = optimise(`<svg />`);
/// ```
///
///
/// Or, provide your own config
///
/// ```js
/// import { optimise } from "@oxvg/wasm";
///
/// // Only optimise path data
/// const result = optimise(`<svg />`, { convertPathData: {} });
/// ```
///
/// Or, extend a preset
///
/// ```js
/// import { optimise, extend, Extends } from "@oxvg/wasm";
///
/// const result = optimise(
///     `<svg />`,
///     extend(Extends.Default, { convertPathData: { removeUseless: false } }),
/// );
/// ```
pub fn optimise(svg: String, config: Option<Jobs>) -> Result<String, Error<Status>> {
  let arena = typed_arena::Arena::new();
  let dom = parse(&svg, &arena).map_err(|e| Error::new(Status::InvalidArg, e.to_string()))?;
  config
    .unwrap_or_default()
    .run(&dom, &Info::<Element>::new(&arena))
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
pub fn extend(extends: Extends, config: Option<Jobs>) -> Jobs {
  match config {
    Some(ref jobs) => extends.extend(jobs),
    None => extends.jobs(),
  }
}

#[allow(clippy::needless_pass_by_value)]
fn generic_error<T: ToString>(err: T) -> Error<Status> {
  Error::new(Status::GenericFailure, err.to_string())
}
