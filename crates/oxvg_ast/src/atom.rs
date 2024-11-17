pub trait Atom: for<'a> From<&'a str> + Into<String> + From<String> {}
