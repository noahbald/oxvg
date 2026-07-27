//! This crate takes SVG documents and transforms them into JSX components, for use in React, Preact,
//! or any other JSX-compatible language.
//!
//! This crate aims to implement SVGR as closely as possible but may not behave exactly the same areas.
//! Intentional differences are listed below.
//! If you believe an SVGR feature is missing or behaves differently, please raise a PR.
//!
//! # Differences to SVGR
//!
//! - Accepts `oxvg` parameter instead of `svgo` for enabling optimisations
//! - Accepts `oxvg_config` parameter instead of `svgo_config` for configuring optimisations
//! - Template variables are strings instead of Babel AST items
//! - Template should return a string instead of Babel AST
//! - Templates aren't parsed or validated, they will be joined as-is
#[cfg(feature = "swc_core")]
pub mod swc;

pub mod config;
pub mod error;
#[cfg(test)]
mod test;
mod utils;

#[cfg(any(feature = "swc_core", feature = "oxc_ast"))]
use std::io::Write;

pub use config::Config;
pub use error::Error;
use oxvg_ast::node::Ref;

use crate::config::{State, TemplateContext};
#[cfg(any(feature = "swc_core", feature = "oxc_ast"))]
use crate::{
    config::{Template, VariablesAST, VariablesString},
    error::{BuildError, TemplateError},
};

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

#[cfg(all(feature = "swc_core", feature = "oxc_ast"))]
compile_error!("Only one of `swc_core` or `oxc_ast` should be enabled");

#[cfg(not(any(feature = "swc_core", feature = "oxc_ast")))]
compile_error!("One of `swc_core` or `oxc_ast` should be enabled");

#[cfg(any(feature = "swc_core", feature = "oxc_ast"))]
/// Writes an SVG document into a JSX document
///
/// # Errors
///
/// If there's an error building the document or parsing the config.
pub fn transform<
    'input,
    'arena,
    W: Write,
    FA: FnOnce(&mut W, VariablesAST, TemplateContext) -> Result<(), TemplateError>,
    FS: FnOnce(&mut W, VariablesString, TemplateContext) -> Result<(), TemplateError>,
>(
    code: Ref<'input, 'arena>,
    #[cfg(feature = "optimise")] allocator: oxvg_ast::arena::Allocator<'input, 'arena>,
    config: Option<Config>,
    state: Option<State>,
    template: Option<Template<W, FA, FS>>,
    w: &mut W,
) -> Result<(), Error<'input>> {
    let config = config.unwrap_or_default();
    #[cfg(feature = "optimise")]
    if config.oxvg.is_none_or(|b| b) {
        use oxvg_optimiser::Jobs;

        match config.oxvg_config.as_ref() {
            Some(jobs) => jobs.run(code, &oxvg_ast::visitor::Info::new(allocator)),
            None => Jobs::default().run(code, &oxvg_ast::visitor::Info::new(allocator)),
        }
        .map_err(Error::OptimiseError)?;
    }
    #[cfg(feature = "swc_core")]
    {
        use std::collections::HashSet;

        let mut jsx =
            crate::swc::to_jsx(code, &config, state.as_ref()).map_err(Error::BuildError)?;
        let mut native_idents = HashSet::new();
        crate::swc::preset(&mut jsx, &config, &mut native_idents)?;
        let options = config.opts(state, native_idents);
        let variables = crate::swc::Variables::new(jsx, &options).map_err(Error::ConfigError)?;
        let context = TemplateContext { options };
        match template {
            Some(Template::AST(template, _)) => template(w, variables, context),
            Some(Template::String(template, _)) => template(w, variables.into(), context),
            None => crate::swc::default_template(w, variables, context),
        }
        .map_err(Error::TemplateError)
    }
}
