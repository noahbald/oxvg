//! XLink prefixed attribute types
use crate::enum_attr;

enum_attr!(
    /// Provides documentation to XLink-aware processors.
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/linking.html#XLinkShowAttribute)
    XLinkShow {
        /// New
        New: "new",
        /// Replace
        Replace: "replace",
        /// Embed
        Embed: "embed",
        /// Other
        Other: "other",
        /// None
        None: "none",
    }
);
