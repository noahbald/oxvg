/*!
The OXVG optimiser is library for optimising SVG documents.

The optimiser should be used with a document from [`oxvg_ast`] which can be processed by the
optimiser's [`Jobs`].

# Example

Parsing and optimising a document

```ignore
use oxvg_ast::{
    implementations::{roxmltree::parse, shared::Element},
    serialize::{Node, Options},
    visitor::Info,
};
use oxvg_optimiser::Jobs;

let mut jobs = Jobs::default();
let arena = typed_arena::Arena::new();
let dom = parse(
    r#"<svg xmlns="http://www.w3.org/2000/svg">
        test
    </svg>"#,
    &arena,
)
.unwrap();
jobs.run(&dom, &Info::<Element>::new(&arena)).unwrap();
dom.serialize_with_options(Options::default()).unwrap();
```
*/

#[macro_use]
extern crate lazy_static;

#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

#[cfg(test)]
mod configuration;
mod jobs;
mod utils;

pub use crate::jobs::*;

#[cfg(test)]
#[ctor::ctor]
fn init_test() {
    let _ = env_logger::builder().is_test(true).try_init();
}
