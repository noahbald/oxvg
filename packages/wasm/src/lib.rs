//! WASM bindings for OXVG
extern crate console_error_panic_hook;
use oxvg_ast::{
    implementations::{roxmltree::parse, shared::Element},
    serialize::{self, Node as _, Options},
    visitor::Info,
};
use oxvg_optimiser::Jobs;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
/// Optimise an SVG document using the provided config
///
/// # Errors
/// - If the document fails to parse
/// - If any of the optimisations fail
/// - If the optimised document fails to serialize
pub fn optimise(svg: &str, config_json: Option<String>) -> Result<String, String> {
    console_error_panic_hook::set_once();

    let mut config = if let Some(config) = config_json {
        serde_json::from_str(&config).map_err(|err| err.to_string())?
    } else {
        Jobs::default()
    };
    let arena = typed_arena::Arena::new();
    let dom = parse(svg, &arena).map_err(|e| e.to_string())?;
    config
        .run(&dom, &Info::<Element>::new(&arena))
        .map_err(|err| err.to_string())?;

    dom.serialize_with_options(Options {
        indent: serialize::Indent::None,
        ..Default::default()
    })
    .map_err(|err| err.to_string())
}
