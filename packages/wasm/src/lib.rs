//! WASM bindings for OXVG
extern crate console_error_panic_hook;
use oxvg_ast::{
    implementations::{roxmltree::parse, shared::Element},
    serialize::{self, Node as _, Options},
    visitor::Info,
};
use oxvg_optimiser::{Extends, Jobs};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
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
/// import { optimise, extend } from "@oxvg/wasm";
///
/// const result = optimise(
///     `<svg />`,
///     extend("default", { convertPathData: { removeUseless: false } }),
/// );
/// ```
pub fn optimise(svg: &str, config: Option<Jobs>) -> Result<String, String> {
    console_error_panic_hook::set_once();

    let arena = typed_arena::Arena::new();
    let dom = parse(svg, &arena).map_err(|e| e.to_string())?;
    config
        .unwrap_or_default()
        .run(&dom, &Info::<Element>::new(&arena))
        .map_err(|err| err.to_string())?;

    dom.serialize_with_options(Options {
        indent: serialize::Indent::None,
        ..Default::default()
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
#[allow(clippy::needless_pass_by_value)]
/// Returns the given config with omitted options replaced with the config provided by `extends`.
/// I.e. acts like `{ ...extends, ...config }`
pub fn extend(extends: &Extends, config: Option<Jobs>) -> Jobs {
    match config {
        Some(ref jobs) => extends.extend(jobs),
        None => extends.jobs(),
    }
}
