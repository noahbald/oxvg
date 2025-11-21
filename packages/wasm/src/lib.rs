//! WASM bindings for OXVG
extern crate console_error_panic_hook;
use oxvg_ast::{
    parse::roxmltree::{parse_with_options, ParsingOptions},
    serialize::Node as _,
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

    let config = config.unwrap_or_default();
    parse_with_options(
        svg,
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
        |dom, allocator| {
            config
                .run(dom, &Info::new(allocator))
                .map_err(|e| e.to_string())?;
            dom.serialize().map_err(|e| e.to_string())
        },
    )
    .map_err(|e| e.to_string())?
}

#[wasm_bindgen(js_name = convertSvgoConfig)]
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
pub fn convert_svgo_config(config: Option<Vec<JsValue>>) -> Result<Jobs, String> {
    if let Some(config) = config {
        let config = config
            .into_iter()
            .map(serde_wasm_bindgen::from_value::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string());
        Jobs::from_svgo_plugin_config(Some(config?)).map_err(|err| err.to_string())
    } else {
        Jobs::from_svgo_plugin_config(None).map_err(|err| err.to_string())
    }
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
