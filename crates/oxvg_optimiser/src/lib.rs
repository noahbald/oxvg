/*!
The OXVG optimiser is library for optimising SVG documents.

The optimiser should be used with a document from [`oxvg_ast`] which can be processed by the
optimiser's [`Jobs`].

# Example

Parsing and optimising a document

```
use oxvg_ast::{
    parse::roxmltree::parse,
    serialize::{Node as _},
    visitor::Info,
};
use oxvg_optimiser::Jobs;

let result: String = parse(
    r#"<svg xmlns="http://www.w3.org/2000/svg">
        test
    </svg>"#,
    |dom, allocator| {
        let jobs = Jobs::default();
        jobs.run(dom, &Info::new(allocator)).unwrap();
        dom.serialize().unwrap()
    }
).unwrap();
```
*/

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

#[cfg(test)]
mod configuration;
pub mod error;
mod jobs;
mod utils;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use crate::jobs::*;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(from_wasm_abi, into_wasm_abi))]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
    #[doc(hidden)]
    #[cfg(feature = "napi")]
    #[cfg_attr(feature = "clap", value(skip))]
    /// Compatibility option for NAPI
    // FIXME: force discriminated union to prevent NAPI from failing CI
    Napi(),
}

impl Extends {
    /// Creates a configuration based on the variant.
    pub fn jobs(&self) -> Jobs {
        match self {
            Extends::None => Jobs::none(),
            Extends::Default => Jobs::default(),
            Extends::Safe => Jobs::safe(),
            #[cfg(feature = "napi")]
            Extends::Napi() => Jobs::none(),
        }
    }

    /// Creates a configuration with the presets jobs extended by the given jobs.
    pub fn extend(&self, jobs: &Jobs) -> Jobs {
        let mut result = self.jobs();
        result.extend(jobs);
        result
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init_test() {
    let _ = env_logger::builder().is_test(true).try_init();
}
