//! XML prefixed attribute types
use crate::enum_attr;

enum_attr!(
    /// Deprecated XML attribute to specify whether white space is preserved in character data.
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/struct.html#XMLSpaceAttribute)
    /// [w3 | SVG 2](https://svgwg.org/svg2-draft/struct.html#XMLSpaceAttribute)
    XmlSpace {
        /// With this value set, whitespace characters will be processed in this order:
        ///
        /// - All newline characters are removed.
        /// - All tab characters are converted into space characters.
        /// - All leading and trailing space characters are removed.
        /// - All contiguous space characters are collapsed into a single space character.
        Default: "default",
        /// This value tells the user agent to convert all newline and tab characters into spaces. Then, it draws all space characters.
        Preserve: "preserve",
    }
);
