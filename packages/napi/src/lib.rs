//! NAPI bindings for OXVG
use napi::{Error, Status};
use oxvg_ast::{
  implementations::markup5ever::{Element5Ever, Node5Ever},
  parse, serialize,
  visitor::Info,
};
use oxvg_optimiser::Jobs;
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
pub fn optimise(svg: String, config_json: Option<String>) -> Result<String, Error<Status>> {
  let config = if let Some(config) = config_json {
    serde_json::from_str(&config).map_err(generic_error)?
  } else {
    Jobs::<Element5Ever>::default()
  };
  let dom: Node5Ever = parse::Node::parse(&svg).map_err(generic_error)?;
  config.run(&dom, &Info::default()).map_err(generic_error)?;

  serialize::Node::serialize(&dom).map_err(generic_error)
}

#[allow(clippy::needless_pass_by_value)]
fn generic_error<T: ToString>(err: T) -> Error<Status> {
  Error::new(Status::GenericFailure, err.to_string())
}
