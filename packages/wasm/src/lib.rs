//! WASM bindings for OXVG
extern crate console_error_panic_hook;
use oxvg_ast::{
    implementations::markup5ever::{Element5Ever, Node5Ever},
    parse, serialize,
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

    let config = if let Some(config) = config_json {
        serde_json::from_str(&config).map_err(|err| err.to_string())?
    } else {
        Jobs::<Element5Ever>::default()
    };
    let dom: Node5Ever = parse::Node::parse(svg).map_err(|err| err.to_string())?;
    config
        .run(&dom, &Info::default())
        .map_err(|err| err.to_string())?;

    serialize::Node::serialize(&dom).map_err(|err| err.to_string())
}
