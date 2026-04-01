//! NAPI bindings for OXVG
use napi::{bindgen_prelude::Unknown, Error, Status};
use oxvg_actions::{Action, ActionNapi, DerivedState, DerivedStateNapi};
use oxvg_ast::{
  arena::Allocator,
  parse::roxmltree::{parse, parse_tree_with_allocator},
  serialize::{Node as _, Options},
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
pub fn optimise(svg: String, config: Option<Jobs>) -> napi::Result<String> {
  let config = config.unwrap_or_default();
  parse(&svg, |dom, allocator| {
    config
      .run(dom, &Info::new(allocator))
      .map_err(generic_error)?;
    dom
      .serialize_with_options(Options::default())
      .map_err(generic_error)
  })
  .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))?
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
pub fn convert_svgo_config(
  #[napi(
    ts_arg_type = "Array<'preset-default' | {[K in keyof Jobs]: K | { name: K, params?: Jobs[K] }}[keyof Jobs]> | undefined | null"
  )]
  config: Option<Vec<Unknown>>,
) -> Result<Jobs, Error<Status>> {
  Jobs::napi_from_svgo_plugin_config(config)
}

#[allow(clippy::needless_pass_by_value)]
fn generic_error<T: ToString>(err: T) -> Error<Status> {
  Error::new(Status::GenericFailure, err.to_string())
}

#[napi]
/// An actor holds a reference to a document to act upon.
///
/// The actor will embed it's state into the document upon parsing and serializing.
#[allow(clippy::struct_field_names)]
pub struct Actor {
  actor: oxvg_actions::Actor<'static, 'static>,
  source_ptr: *mut str,
  xml_ptr: *mut roxmltree::Document<'static>,
  arena_ptr: *mut oxvg_ast::arena::Arena<'static, 'static>,
  values_ptr: *mut oxvg_ast::arena::Values,
}

#[napi]
impl Actor {
  #[napi(constructor)]
  /// Creates a new actor with a reference to the document. The state of the actor will be
  /// derived from the document's `oxvg:state` element.
  ///
  /// # Errors
  ///
  /// If parsing fails
  pub fn new(document: String) -> napi::Result<Self> {
    let source = Box::leak(document.into_boxed_str());
    let source_ptr = std::ptr::from_mut(source);
    let xml = Box::leak(Box::new(
      roxmltree::Document::parse(source)
        .map_err(|err| oxvg_actions::Error::ParseError(err.to_string()))
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?,
    ));
    let xml_ptr = std::ptr::from_mut(xml);

    let values = Box::leak(Box::new(Allocator::new_values()));
    let arena = Box::leak(Box::new(Allocator::new_arena_with_capacity(
      xml.descendants().len(),
    )));
    let values_ptr = std::ptr::from_mut(values);
    let arena_ptr = std::ptr::from_mut(arena);

    let (root, arena) = parse_tree_with_allocator(xml, arena, values, |root, arena| (root, arena))
      .map_err(|err| oxvg_actions::Error::ParseError(err.to_string()))
      .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?;

    Ok(Self {
      actor: oxvg_actions::Actor::new(root, arena)
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?,
      source_ptr,
      xml_ptr,
      arena_ptr,
      values_ptr,
    })
  }

  /// Returns a rich state object based on the `oxvg:state` embedded in the document
  ///
  /// # Errors
  ///
  /// If the state is invalid
  #[napi]
  pub fn derive_state(&self) -> napi::Result<DerivedStateNapi> {
    self
      .actor
      .derive_state()
      .as_ref()
      .map(DerivedState::to_napi)
      .map_err(generic_error)
  }

  /// Executes the given action and it's arguments upon the document.
  ///
  /// # Errors
  ///
  /// If the action fails
  #[napi]
  pub fn dispatch(&mut self, action: ActionNapi) -> napi::Result<()> {
    self
      .actor
      .dispatch(Action::from_napi(action))
      .map_err(generic_error)
  }

  /// Sets the attribute to selected elements.
  ///
  /// # Errors
  ///
  /// When root element is missing.
  #[napi]
  #[allow(clippy::needless_pass_by_value)]
  pub fn attr(&mut self, name: String, value: String) -> napi::Result<()> {
    self.actor.attr(&name, &value).map_err(generic_error)
  }

  /// Removes OXVG state from the document
  #[napi]
  pub fn forget(&mut self) {
    self.actor.forget();
  }

  /// Updates the state of the actor to point to the elements matching the given selector.
  /// Elements can also be selected by a space/comma separated list of allocation-id
  /// integers.
  ///
  /// # Errors
  ///
  /// If the query is invalid
  #[napi]
  #[allow(clippy::needless_pass_by_value)]
  pub fn select(&mut self, query: String) -> napi::Result<()> {
    self.actor.select(&query).map_err(generic_error)
  }

  /// Updates the state of the actor to point to the elements matching the given selector,
  /// including any previous selections.
  /// Elements can also be selected by a space/comma separated list of allocation-id
  /// integers.
  ///
  /// # Errors
  ///
  /// If the query is invalid
  #[napi]
  #[allow(clippy::needless_pass_by_value)]
  pub fn select_more(&mut self, query: String) -> napi::Result<()> {
    self.actor.select_more(&query).map_err(generic_error)
  }

  /// Updates the state of the actor to deselected any selected nodes.
  #[napi]
  pub fn deselect(&mut self) {
    self.actor.deselect();
  }

  /// Returns the actor's updated document as a string
  ///
  /// # Errors
  ///
  /// If serialization fails
  #[napi]
  pub fn document(&self) -> napi::Result<String> {
    self.actor.root.serialize().map_err(generic_error)
  }
}

impl Drop for Actor {
  fn drop(&mut self) {
    unsafe {
      drop(Box::from_raw(self.source_ptr));
      drop(Box::from_raw(self.xml_ptr));
      drop(Box::from_raw(self.arena_ptr));
      drop(Box::from_raw(self.values_ptr));
    }
  }
}
