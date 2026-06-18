//! Methods for optimizing SVG paths

use crate::{
    paths::segment::{self},
    Path,
};

pub use crate::paths::segment::Tolerance;

bitflags! {
    /// Options for which operations should be applied during optimisation
    #[derive(Copy, Clone, Debug)]
    pub struct Options: u16 {
        /// Convert all segments to closed segments.
        ///
        /// - `fill`: Safe
        /// - `fill-rule: "evenodd"`: Safe
        /// - `fill-rule: "nonezero"`: Safe
        /// - `stroke`: Unsafe
        /// - `stroke`
        ///      + `stroke-linecap: "round"/"square"`
        ///      + `stroke-linejoin: "round"`: Safe
        /// - `marker-start`: Safe
        /// - `marker-mid`: Unsafe
        /// - `marker-end`: Unsafe
        const CloseSegments = 1 << 0;
        /// Unite overlapping closed segments.
        ///
        /// - `fill`: Safe
        /// - `fill-rule: "evenodd"`: Unsafe
        /// - `fill-rule: "nonzero"`: Safe
        /// - `stroke`: Unsafe
        /// - `marker-*`: Unsafe
        const XORSegments = 1 << 1;
        /// XOR overlapping closed segments.
        ///
        /// - `fill`: Safe
        /// - `fill-rule: "evenodd"`: Safe
        /// - `fill-rule: "nonzero"`: Unsafe
        /// - `stroke`: Unsafe
        /// - `marker-*`: Unsafe
        const UniteSegments = 1 << 2;
        /// Join commands that fit within the path of the previous and next command.
        ///
        /// - `fill`: Safe
        /// - `fill-rule: "evenodd"`: Safe
        /// - `fill-rule: "nonzero"`: Safe
        /// - `stroke`: Safe
        /// - `marker-start`: Safe
        /// - `marker-mid`: Unsafe
        /// - `marker-end`: Safe
        const JoinNodes = 1 << 3;
        /// Remove move commands immediately followed by another move command.
        ///
        /// - `fill`: Safe
        /// - `fill-rule: "evenodd"`: Safe
        /// - `fill-rule: "nonzero"`: Safe
        /// - `stroke`: Safe
        /// - `marker-*`: Unsafe
        const RemoveEmptySegments = 1 << 4;
        /// Remove commands where all command args are effectively zero.
        ///
        /// - `fill`: Safe
        /// - `fill-rule: "evenodd"`: Safe
        /// - `fill-rule: "nonzero"`: Safe
        /// - `stroke` + `stroke-linecap`: Unsafe
        /// - `marker-*`: Safe
        const RemoveNoopCommands = 1 << 5;
        /// Replace final line commands of segments that return to the start with `Z`.
        /// Remove `Z` when retained final commands of segments return to the start.
        ///
        /// - `fill`: Safe
        /// - `fill-rule: "evenodd"`: Safe
        /// - `fill-rule: "nonzero"`: Safe
        /// - `stroke`: Unsafe
        /// - `marker-start`: Safe
        /// - `marker-mid`: Unsafe
        /// - `marker-end`: Unsafe
        const RemoveCloseLine = 1 << 6;
        /// Convert effectively straight curves and arcs into lines.
        ///
        /// - `*`: Safe
        const StraightCurves = 1 << 7;
        /// Converts effectively arced curves into arcs.
        ///
        /// - `*`: Safe
        const ArcCurves = 1 << 8;
        /// Rounds the radius of the arc weighted by the chord distance.
        ///
        /// - `*`: Safe
        const SmartArcRounding = 1 << 9;

        /// A set of flags that should be excluded when `stroke` is present
        const UnsafeStroke = Self::CloseSegments.union(Self::UniteSegments).union(Self::RemoveCloseLine).bits();
        /// A set of flags that should be excluded when `fill: evenodd` is present
        const UnsafeEvenOdd = Self::UniteSegments.bits();
        /// A set of flags that should be excluded when `fill: nonzero` is present
        const UnsafeNonZero = Self::XORSegments.bits();
        /// A set of flags that should be excluded when `marker-*` is present
        const UnsafeMarker = Self::UniteSegments.union(Self::RemoveEmptySegments).bits();
        /// A set of flags that should be excluded when `marker-start` is present
        const UnsafeMarkerStart = Self::UnsafeMarker.bits();
        /// A set of flags that should be excluded when `marker-mid` is present
        const UnsafeMarkerMid = Self::UnsafeMarker.union(Self::CloseSegments).union(Self::JoinNodes).union(Self::RemoveCloseLine).bits();
        /// A set of flags that should be excluded when `marker-end` is present
        const UnsafeMarkerEnd = Self::UnsafeMarker.union(Self::CloseSegments).union(Self::RemoveCloseLine).bits();
        /// A set of flags that should be excluded when `stroke` + `stroke-linecap` is present
        const UnsafeStrokeLinecap = Self::RemoveNoopCommands.bits();
    }
}

impl Default for Options {
    fn default() -> Self {
        Options::empty()
    }
}

impl Path {
    // TODO: Optimisation options based on `StyleInfo`
    /// Returns an optimised version of the input path
    ///
    /// Note that depending on the options and style-info given, the optimisation may be lossy.
    ///
    /// # Examples
    ///
    /// If you don't have any access to attributes or styles for a specific SVG element the path
    /// belongs to, try running this with the conservative approach
    ///
    /// ```
    /// use oxvg_path::Path;
    /// use oxvg_path::optimize::{Options, Tolerance};
    /// use oxvg_path::parser::Parse as _;
    ///
    /// let mut path = Path::parse_string("M 10,30 L 10,50 L 30 30 H 10").unwrap();
    /// let options = Options::default();
    ///
    /// path = path.optimize(options, &Tolerance::default());
    /// assert_eq!(&path.to_string(), "M10 30v20l20-20H10Z");
    /// ```
    #[must_use]
    pub fn optimize(&self, options: Options, tolerance: &Tolerance) -> Path {
        let mut segments = segment::Path::from_svg(self, tolerance);

        if options.contains(Options::CloseSegments) {
            segments.close_segments();
        }

        if options.contains(Options::UniteSegments) {
            segments = segments.non_zero(tolerance);
        }
        if options.contains(Options::XORSegments) {
            segments = segments.even_odd(tolerance);
        }

        segments.simplify(options, tolerance);

        segments.to_svg(tolerance, options.contains(Options::SmartArcRounding))
    }
}
