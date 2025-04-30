//! NAPI bindings for OXVG
use napi::{Error, Status};
use oxvg_ast::{
  implementations::{roxmltree::parse, shared::Element},
  serialize::{self, Node as _, Options},
  visitor::Info,
};
use oxvg_optimiser::Jobs;
#[macro_use]
extern crate napi_derive;

#[napi(string_enum)]
#[derive(Default)]
/// A preset which the specified jobs can overwrite
pub enum Extends {
  /// A preset that contains no jobs.
  None,
  /// The default preset.
  /// Uses [`oxvg_optimiser::Jobs::default`]
  #[default]
  Default,
  /// The correctness preset. Produces a preset that is less likely to
  /// visually change the document.
  /// Uses [`oxvg_optimiser::Jobs::correctness`]
  Safe,
  // TODO: File(Path),
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
/// Optimise an SVG document using the provided config.
///
/// The config extends the default preset when `extends` is unspecified.
///
/// # Errors
/// - If the document fails to parse
/// - If any of the optimisations fail
/// - If the optimised document fails to serialize
pub fn optimise(
  svg: String,
  config: Option<Jobs>,
  extend: Option<Extends>,
) -> Result<String, Error<Status>> {
  let arena = typed_arena::Arena::new();
  let mut jobs = match extend.unwrap_or_default() {
    Extends::None => Jobs::none(),
    Extends::Default => Jobs::default(),
    Extends::Safe => Jobs::safe(),
  };
  if let Some(ref config) = config {
    jobs.extend(config);
  }
  let dom = parse(&svg, &arena).map_err(|e| Error::new(Status::InvalidArg, e.to_string()))?;
  jobs
    .run(&dom, &Info::<Element>::new(&arena))
    .map_err(generic_error)?;

  dom
    .serialize_with_options(Options {
      indent: serialize::Indent::None,
      ..Default::default()
    })
    .map_err(generic_error)
}

#[allow(clippy::needless_pass_by_value)]
fn generic_error<T: ToString>(err: T) -> Error<Status> {
  Error::new(Status::GenericFailure, err.to_string())
}
