//! Animation addition attributes as specified in [animations](https://svgwg.org/specs/animations/#AdditionAttributes)
use crate::enum_attr;

enum_attr!(
    #[derive(Default)]
    /// Controls whether the animation is cumulative
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#AdditiveAttribute)
    /// [w3 | animations](https://svgwg.org/specs/animations/#AdditiveAttribute)
    Accumulate {
        #[default]
        /// Repeat iterations are not cumulative
        None: "none",
        /// Repeat iterations build upon previous iterations
        Sum: "sum",
    }
);
enum_attr!(
    #[derive(Default)]
    /// Controls whether the animation is additive.
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/animate.html#AdditiveAttribute)
    /// [w3 | SVG 2](https://svgwg.org/specs/animations/#AdditiveAttribute)
    Additive {
        #[default]
        /// Will override the current value or other lower priority animations
        Replace: "replace",
        /// Will add to the current value or other lower priority animations
        Sum: "sum",
    }
);
