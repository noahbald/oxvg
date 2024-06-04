#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod configuration;
mod jobs;

pub use crate::jobs::*;
