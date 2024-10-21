#[macro_use]
extern crate lazy_static;

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
