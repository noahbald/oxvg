//! `XLink` prefixed attribute types
use crate::enum_attr;

enum_attr!(
    /// Used to communicate the desired timing of traversal from the starting resource to the ending resource.
    XLinkActuate {
        /// Traverse from the starting resource to the ending resource only on a post-loading event triggered for the purpose of traversal.
        OnRequest: "onRequest",
        /// Traverse to the ending resource immediately on loading the starting resource.
        OnLoad: "onLoad",
    }
);

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

enum_attr!(
    /// Identifies the type of XLink being used..
    XLinkType {
        /// Associates the local resource with one remote resource.
        Simple: "simple",
    }
);
