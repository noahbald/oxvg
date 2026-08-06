//! [`segment::Path`] wrapped with fill-rule, for use with boolean path operations.
use crate::paths::segment;

/// [`segment::Path`] wrapped with fill-rule, for use with boolean path operations.
pub struct Path {
    /// The path to be operated upon.
    pub inner: segment::Path,
    /// Whether the fill-rule is evenodd or nonzero.
    pub evenodd: bool,
}
