//! Methods for optimizing SVG paths

use crate::{
    paths::segment::{self, Tolerance},
    Path,
};

bitflags! {
    /// Options for which operations should be applied during optimisation
    #[derive(Copy, Clone)]
    pub struct Options: u8 {
        /// Convert all segments to closed segments.
        ///
        /// - `fill`: Safe
        /// - `stroke`: Unsafe
        /// - `marker-start`: Safe
        /// - `marker-mid`: Unsafe
        /// - `marker-end`: Unsafe
        const CloseSegments = 1 << 0;
        /// Unite overlapping closed segments.
        ///
        /// - `fill`: Safe
        /// - `stroke`: Unsafe
        /// - `marker-*`: Unsafe
        const UnionSegments = 1 << 1;
        /// Join commands that fit within the path of the previous and next command.
        ///
        /// - `fill`: Safe
        /// - `stroke`: Safe
        /// - `marker-start`: Safe
        /// - `marker-mid`: Unsafe
        /// - `marker-end`: Safe
        const JoinNodes = 1 << 2;
        /// Remove move commands immediately followed by another move command.
        ///
        /// - `fill`: Safe
        /// - `stroke`: Safe
        /// - `marker-*`: Unsafe
        const RemoveEmptySegments = 1 << 3;
        /// Remove final line commands of segments that return to the start.
        ///
        /// - `fill`: Safe
        /// - `stroke`: Unsafe
        /// - `marker-start`: Safe
        /// - `marker-mid`: Unsafe
        /// - `marker-end`: Unsafe
        const RemoveCloseLine = 1 << 4;

        /// A set of flags that should be excluded when `stroke` is present
        const UnsafeStroke = Self::CloseSegments.union(Self::UnionSegments).union(Self::RemoveCloseLine).bits();
        /// A set of flags that should be excluded when `marker-*` is present
        const UnsafeMarker = Self::UnionSegments.union(Self::RemoveEmptySegments).bits();
        /// A set of flags that should be excluded when `marker-start` is present
        const UnsafeMarkerStart = Self::UnsafeMarker.bits();
        /// A set of flags that should be excluded when `marker-mid` is present
        const UnsafeMarkerMid = Self::UnsafeMarker.union(Self::CloseSegments).union(Self::JoinNodes).union(Self::RemoveCloseLine).bits();
        /// A set of flags that should be excluded when `marker-end` is present
        const unsafeMarkerEnd = Self::UnsafeMarker.union(Self::CloseSegments).union(Self::RemoveCloseLine).bits();
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
    /// use oxvg_path::optimize::{optimize, Options}
    /// use oxvg_path::parser::Parse as _;
    ///
    /// let mut path = Path::parse_string("M 10,50 L 10,50").unwrap();
    /// let options = Options::empty();
    ///
    /// run(&mut path, &options, &style_info);
    /// assert_eq!(&path.to_string(), "M10 50h0");
    /// ```
    pub fn optimize(&self, options: Options, tolerance: &Tolerance) -> Path {
        let mut segments = segment::Path::from_svg(self, &tolerance);

        if options.contains(Options::CloseSegments) {
            for segment in segments.0.iter_mut() {
                segment.closed = true
            }
        }

        if options.contains(Options::UnionSegments) {
            // TODO: Boolean union each closed segment
            //       - Skip when markers are present
        }

        segments.simplify(options, &tolerance);

        segments.to_svg(&tolerance)
    }
}
