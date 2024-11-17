use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    NoElementInDocument,
}

pub trait Node: Sized {
    /// # Errors
    ///
    /// Any error cause by the underlying parser, or [Error]
    fn parse(source: &str) -> anyhow::Result<Self>;
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoElementInDocument => f.write_str("No element in document"),
        }
    }
}
