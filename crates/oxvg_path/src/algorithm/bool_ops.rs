//! A reimplementation of [`geo::algorithm::bool_ops`] with regard to SVG.
//!
//! Unlike [`geo`] the output polygon is converted back to [`Self`] by reconstructing
//! from the input path data.
//!
//! For a polygonal output, try building [`geo::geometry::MultiPolygon`] by iterating through
//! the [`segment::Data`] and pushing [`geo::CoordsIter`], then use [`geo::algorithm::bool_ops`].
pub(crate) mod i_overlay_integration;

use geo::{Winding, winding_order::WindingOrder};
use i_overlay::{
    core::{fill_rule::FillRule, solver::Solver},
    float::{
        overlay::{FloatOverlay, OverlayOptions},
        simplify::SimplifyShape,
        single::SingleFloatOverlay as _,
    },
};

use crate::{
    algorithm::bool_ops::i_overlay_integration::{
        BoolOpsCoord,
        convert::{flatten_segment, segment_path_from_shapes},
    },
    paths::{bool, segment},
};

pub use i_overlay::core::overlay_rule::OverlayRule;

impl bool::Path {
    /// Wraps the path with the associated fill-rule.
    pub fn new(path: segment::Path, fill_rule: FillRule) -> Self {
        Self {
            inner: path,
            evenodd: matches!(fill_rule, FillRule::EvenOdd),
        }
    }

    /// Wraps the path with a nonzero fill-rule.
    pub fn nonzero(path: segment::Path) -> Self {
        Self {
            inner: path,
            evenodd: false,
        }
    }

    /// Wraps the path with an evenodd fill-rule.
    pub fn evenodd(path: segment::Path) -> Self {
        Self {
            inner: path,
            evenodd: true,
        }
    }

    /// Returns the overlapping regions shared by both `self` and `other`.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> segment::Path {
        self.boolean_op(other, OverlayRule::Intersect)
    }
    /// Combines the regions of both `self` and `other` into a single geometry, removing
    /// overlaps and merging boundaries. Consider using [`unary_union`] for efficiently combining several adjacent / overlapping geometries.
    #[must_use]
    pub fn union(&self, other: &Self) -> segment::Path {
        self.boolean_op(other, OverlayRule::Union)
    }

    /// The regions that are in either `self` or `other`, but not in both.
    #[must_use]
    pub fn xor(&self, other: &Self) -> segment::Path {
        self.boolean_op(other, OverlayRule::Xor)
    }

    /// The regions of `self` which are not in `other`.
    #[must_use]
    pub fn difference(&self, other: &Self) -> segment::Path {
        self.boolean_op(other, OverlayRule::Difference)
    }

    /// Performs a boolean operation between the shapes. The returned path's fill-rule is
    /// always [`FillRule::NonZero`].
    ///
    /// Unlike [`geo`] the output polygon is converted back to [`Self`] by reconstructing
    /// from the input path data.
    ///
    /// See [`geo::algorithm::bool_ops::BooleanOps::boolean_op`]
    #[must_use]
    pub fn boolean_op(&self, other: &Self, op: OverlayRule) -> segment::Path {
        boolean_op([self, other], op)
    }
}

/// Completes boolean operation over many adjacent / overlapping paths.
///
/// The resulting path always has `nonzero` fill-rule.
pub fn boolean_op<'a>(
    boppables: impl IntoIterator<Item = &'a bool::Path>,
    op: OverlayRule,
) -> segment::Path {
    let mut registry = vec![];
    let mut subject = boppables.into_iter().map(|path| {
        let contours = path
            .inner
            .0
            .iter()
            .map(|ring| flatten_segment(ring, &mut registry))
            .collect::<Vec<_>>();
        contours.simplify_shape(if path.evenodd {
            FillRule::EvenOdd
        } else {
            FillRule::NonZero
        })
    });

    let Some(mut shapes) = subject.next() else {
        return segment::Path(vec![]);
    };
    for next in subject {
        shapes = shapes.overlay(&next, op, FillRule::NonZero);
    }
    segment_path_from_shapes(shapes, registry)
}

/// Efficient [union](bool::Path::union) of many adjacent / overlapping geometries
///
/// This is typically much faster than `union`ing a bunch of geometries together one at a time.
///
/// Note: Geometries can be wound in either direction, but the winding order must be consistent,
/// and each polygon's interior rings must be wound opposite to its exterior.
///
/// See [`geo::algorithm::bool_ops::unary_union`].
pub fn unary_union<'a>(boppables: impl IntoIterator<Item = &'a bool::Path>) -> segment::Path {
    let mut winding_order: Option<WindingOrder> = None;
    let mut registry = vec![];
    let subject = boppables
        .into_iter()
        .flat_map(|boppable| {
            boppable
                .inner
                .0
                .iter()
                .map(|ring| {
                    let shape = flatten_segment(ring, &mut registry);
                    if winding_order.is_none() {
                        winding_order = BoolOpsCoord::line_string(&shape).winding_order();
                    }
                    shape
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let fill_rule = if winding_order == Some(WindingOrder::CounterClockwise) {
        FillRule::Positive
    } else {
        FillRule::Negative
    };

    let shapes = FloatOverlay::with_subj_custom(&subject, OverlayOptions::ogc(), Solver::default())
        .overlay(OverlayRule::Subject, fill_rule);
    segment_path_from_shapes(shapes, registry)
}

#[cfg(test)]
mod test {
    use oxvg_parse::Parse;

    use crate::{
        Path,
        geometry::Tolerance,
        paths::{bool, segment},
    };

    #[test]
    fn intersect_evenodd() {
        let background = Path::parse_string("M 2 2h8v8H2z").unwrap();
        let foreground = Path::parse_string(
            "M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z",
        )
        .unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::evenodd(segment::Path::from_svg(&foreground, tolerance));

        let output = background
            .intersection(&foreground)
            .to_svg(tolerance, false);
        assert_eq!(
            &output.to_string(),
            "M5.833 10a4 4 0 0 1 4.165-4.168l.001.497a3.5 3.5 0 0 0-3.67 3.67Z"
        );
    }

    #[test]
    fn unite_evenodd() {
        let background = Path::parse_string("M 2 2h8v8H2z").unwrap();
        let foreground = Path::parse_string(
            "M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z",
        )
        .unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::evenodd(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(
            &output.to_string(),
            "M2 10V2h8v3.832A4 4 0 1 1 5.833 10ZM6.33 10A3.5 3.5 0 1 0 10 6.329V10Z"
        );
    }

    #[test]
    fn subtract_evenodd() {
        let background = Path::parse_string("M 2 2h8v8H2z").unwrap();
        let foreground = Path::parse_string(
            "M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z",
        )
        .unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::evenodd(segment::Path::from_svg(&foreground, tolerance));

        let output = background.difference(&foreground).to_svg(tolerance, false);
        assert_eq!(
            &output.to_string(),
            "M2 10V2h8v3.832a4 4 0 0 0-4.167 4.166ZM6.33 9.997a3.5 3.5 0 0 1 3.669-3.668L10 10H6.33Z"
        );
    }

    #[test]
    fn xor_evenodd() {
        let background = Path::parse_string("M 2 2h8v8H2z").unwrap();
        let foreground = Path::parse_string(
            "M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z",
        )
        .unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::evenodd(segment::Path::from_svg(&foreground, tolerance));

        let output = background.xor(&foreground).to_svg(tolerance, false);
        assert_eq!(
            &output.to_string(),
            "M2 10V2h8v3.832a4 4 0 0 0-4.167 4.166ZM5.834 10.025A4 4 0 0 1 5.833 10h.497A3.5 3.5 0 1 0 10 6.329v-.497a4 4 0 1 1-4.152 4.388ZM6.33 9.997a3.5 3.5 0 0 1 3.669-3.668L10 10H6.33Z"
        );
    }

    #[test]
    fn unite_squares_aligned_winding() {
        let background = Path::parse_string("M0,0 L0,10 L10,10 L10,0 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(&output.to_string(), "M0 10V0h10v5h5v10H5v-5Z");
    }

    #[test]
    fn unite_squares_opposite_winding() {
        let background = Path::parse_string("M0,0 L10,0 L10,10 L0,10 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(output.to_string(), "M0 10V0h10v5h5v10H5v-5Z");
    }

    #[test]
    fn unite_curves_aligned_winding() {
        let background = Path::parse_string("m10 50Q25 25 40 50T50 90 T 10 50").unwrap();
        let foreground = Path::parse_string("m20 60Q35 35 50 60T60 100 T 20 60").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M10.545 50.855Q10.273 50.43 10 50q14.5-24.167 29-1.611Q44.5 50.833 50 60q15 25 10 40-3.125 9.375-17.969-8.594-9.902-6.855-30.945-39.705Z"
        );
    }

    #[test]
    fn unite_curves_opposite_winding() {
        let background = Path::parse_string("m10 50Q25 25 40 50T50 90 T 10 50").unwrap();
        let foreground = Path::parse_string("m60 70Q40 20 10 70T60 70").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M1.826 90.215Q1.563 84.063 10 70q3.344-5.573 6.564-9.904Q13.426 55.384 10 50q13.321-22.202 26.642-4.97Q49.658 44.144 60 70q-4.861 3.038-9.316 5.707Q52.091 83.727 50 90q-2.815 8.444-15.135-5.294-32.062 17.061-33.017 5.844Z"
        );
    }

    #[test]
    fn unite_arc_aligned_winding() {
        let background = Path::parse_string("M10 5a5 5 0 1 0 -10 0a5 5 0 1 0 10 0").unwrap();
        let foreground = Path::parse_string("M10 10a5 5 0 1 0 -10 0a5 5 0 1 0 10 0").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M.002 5.123A5 5 0 0 1 0 5 1 1 0 0 1 10 5a5 5 0 0 1-.67 2.5A5 5 0 0 1 10 10 1 1 0 0 1 0 10 5 5 0 0 1 .67 7.5 5 5 0 0 1 .006 5.245Z"
        );
    }

    #[test]
    fn unite_arc_opposite_winding() {
        let background = Path::parse_string("M0 5a5 5 0 1 0 10 0a5 5 0 1 0 -10 0").unwrap();
        let foreground = Path::parse_string("M10 10a5 5 0 1 0 -10 0a5 5 0 1 0 10 0").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M.002 5.123A5 5 0 0 1 0 5 1 1 0 0 1 10 5a5 5 0 0 1-.67 2.5A5 5 0 0 1 10 10 1 1 0 0 1 0 10 5 5 0 0 1 .67 7.5 5 5 0 0 1 .006 5.245Z"
        );
    }

    #[test]
    fn unite_various_aligned_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground =
            Path::parse_string("M5 10L10 0C20 10 10 10 13 13a5 5 0 0 1 -5 -5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M5 5.061A5 5 0 0 1 5 5h2.5L10 0c2.049 2.049 3.259 3.679 3.912 5H15c2.712 2.712 1.011 3.953-1.114 4.521-.909 1.362-2.258 2.106-.886 3.479a5 5 0 0 1-4.702-3.298 5 5 0 0 1-1.526-.883L5 10 6 8a5 5 0 0 1-.999-2.877Z"
        );
    }

    #[test]
    fn unite_various_opposite_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground = Path::parse_string("M10 10C15 10 10 5 15 7.5a5 5 0 1 0 -5 0h-5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M5 5.061A5 5 0 0 1 5 5h2.848a5 5 0 1 1 8.331 1.556c1.453 3.086-4.659 3.407-5.945 3.44q-.085.003-.175.004C10.02 10 10 10 10 10a5 5 0 0 1-4-2l-1-.5h.67a5 5 0 0 1-.668-2.377Z"
        );
    }

    #[test]
    fn intersect_various_aligned_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground =
            Path::parse_string("M5 10L10 0C20 10 10 10 13 13a5 5 0 0 1 -5 -5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background
            .intersection(&foreground)
            .to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M6.021 8.028A5 5 0 0 1 6 8l1.5-3h6.412c1.077 2.176.645 3.517-.026 4.521C12.093 10 10 10 10 10a5 5 0 0 1-1.702-.298A5 5 0 0 1 8 8l-1.228.819a5 5 0 0 1-.714-.742Z"
        );
    }

    #[test]
    fn intersect_various_opposite_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground = Path::parse_string("M10 10C15 10 10 5 15 7.5a5 5 0 1 0 -5 0h-5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = bool::Path::nonzero(segment::Path::from_svg(&background, tolerance));
        let foreground = bool::Path::nonzero(segment::Path::from_svg(&foreground, tolerance));

        let output = background
            .intersection(&foreground)
            .to_svg(tolerance, false);
        assert_eq!(
            output.to_string(),
            "M5.68 7.518a5 5 0 0 1-.01-.018H10A5 5 0 0 1 7.847 5H15c.584.584.963 1.1 1.178 1.555A5 5 0 0 1 15 7.5c-4.92-2.46-.158 2.341-4.766 2.496-.076.002-.135.003-.175.003Q10.03 10 10 10L6 8a5 5 0 0 1-.289-.43Z"
        );
    }
}
