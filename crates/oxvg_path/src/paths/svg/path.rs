use crate::command;

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
/// A path is a set of commands
///
/// # Example
///
/// Out of the box, parsing and serializing a path will produce optimal formatting
///
/// ```
/// use oxvg_path::{Path, parser::Parse as _};
///
/// let path = Path::parse_string("M 10 0.01 L 0.5 -1").unwrap();
/// assert_eq!(&path.to_string(), "M10 .01.5-1");
/// ```
///
/// For more extensive minification, look into using the [run](convert::run) function.
pub struct Path(pub Vec<command::Data>);
