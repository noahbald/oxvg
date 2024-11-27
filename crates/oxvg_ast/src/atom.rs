use std::fmt::Display;

pub trait Atom:
    Eq
    + Display
    + PartialEq
    + std::fmt::Debug
    + Clone
    + Default
    + for<'a> From<&'a str>
    + Into<String>
    + From<String>
    + AsRef<str>
    + 'static
{
    /// Extracts the string slice of the atom
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}
