pub trait Atom:
    Eq
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
}
