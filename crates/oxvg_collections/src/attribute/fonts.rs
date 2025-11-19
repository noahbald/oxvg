//! Font attributes as specified in [fonts](https://www.w3.org/TR/2011/REC-SVG11-20110816/fonts.html)

use crate::enum_attr;

enum_attr!(
    /// For arabic glyphs, indicates which of the four possible forms this glyph represents
    ///
    /// [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/fonts.html#GlyphElementArabicFormAttribute)
    ArabicForm {
        /// Initial
        Initial: "initial",
        /// Medial
        Medial: "medial",
        /// Terminal
        Terminal: "terminal",
        /// Isolated
        Isolated: "isolated",
    }
);

enum_attr!(
    /// Indicates that the given glyph is only to be used for a particular inline-progression-direction.
    ///
    /// [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/fonts.html#GlyphElementOrientationAttribute)
    Orientation {
        /// Horizontal
        H: "h",
        /// Vertical
        V: "v",
    }
);
