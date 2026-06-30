//! A reimplementation of `geo::algorithm::bool_ops` with regard to SVG. Instead of converting
//! to a polygon and losing curves/arcs, we track which parts of the output belong to the input
//! and rebuild a path based on the source.

pub(crate) mod i_overlay_integration;

use geo::{winding_order::WindingOrder, Winding};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::overlay::{FloatOverlay, OverlayOptions},
};

use crate::{
    algorithm::bool_ops::i_overlay_integration::{
        convert::{ring_to_shape_path, segment_path_from_shapes},
        BoolOpsCoord,
    },
    paths::segment,
};

impl segment::Path {
    /// See [geo::algorithm::bool_ops::BooleanOps::boolean_op]
    pub fn boolean_op(&self, other: &Self, op: OverlayRule) -> Self {
        self.boolean_op_with_fill_rule(other, op, FillRule::EvenOdd)
    }

    pub fn boolean_op_with_fill_rule(
        &self,
        other: &Self,
        op: OverlayRule,
        fill_rule: FillRule,
    ) -> Self {
        let subject = self.0.iter().map(ring_to_shape_path).collect::<Vec<_>>();
        let clip = other.0.iter().map(ring_to_shape_path).collect::<Vec<_>>();
        let shapes = FloatOverlay::with_subj_and_clip_custom(
            &subject,
            &clip,
            OverlayOptions::ogc(),
            Default::default(),
        )
        .overlay(op, fill_rule);
        segment_path_from_shapes(shapes)
    }

    /// Returns the overlapping regions shared by both `self` and `other`.
    pub fn intersection(&self, other: &Self) -> Self {
        self.boolean_op(other, OverlayRule::Intersect)
    }

    /// Returns the overlapping regions shared by both `self` and `other`, using the specified fill rule.
    pub fn intersection_with_fill_rule(&self, other: &Self, fill_rule: FillRule) -> Self {
        self.boolean_op_with_fill_rule(other, OverlayRule::Intersect, fill_rule)
    }

    /// Combines the regions of both `self` and `other` into a single geometry, removing
    /// overlaps and merging boundaries. Consider using [`unary_union`] for efficiently combining several adjacent / overlapping geometries.
    pub fn union(&self, other: &Self) -> Self {
        self.boolean_op(other, OverlayRule::Union)
    }

    /// Combines the regions of both `self` and `other` into a single geometry, removing
    /// overlaps and merging boundaries, using the specified fill rule.
    pub fn union_with_fill_rule(&self, other: &Self, fill_rule: FillRule) -> Self {
        self.boolean_op_with_fill_rule(other, OverlayRule::Union, fill_rule)
    }

    /// The regions that are in either `self` or `other`, but not in both.
    pub fn xor(&self, other: &Self) -> Self {
        self.boolean_op(other, OverlayRule::Xor)
    }

    /// The regions that are in either `self` or `other`, but not in both.
    pub fn xor_with_fill_rule(&self, other: &Self, fill_rule: FillRule) -> Self {
        self.boolean_op_with_fill_rule(other, OverlayRule::Xor, fill_rule)
    }

    /// The regions of `self` which are not in `other`.
    pub fn difference(&self, other: &Self) -> Self {
        self.boolean_op(other, OverlayRule::Difference)
    }

    /// The regions of `self` which are not in `other`, using the specified fill rule.
    pub fn difference_with_fill_rule(&self, other: &Self, fill_rule: FillRule) -> Self {
        self.boolean_op_with_fill_rule(other, OverlayRule::Difference, fill_rule)
    }
}

/// See [`geo::algorithm::bool_ops::unary_union`].
pub fn unary_union<'a>(boppables: impl IntoIterator<Item = &'a segment::Path>) -> segment::Path {
    let mut winding_order: Option<WindingOrder> = None;
    let subject = boppables
        .into_iter()
        .flat_map(|boppable| {
            boppable
                .0
                .iter()
                .map(|ring| {
                    let shape = ring_to_shape_path(ring);
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

    let shapes =
        FloatOverlay::with_subj_custom(&subject, OverlayOptions::ogc(), Default::default())
            .overlay(OverlayRule::Subject, fill_rule);
    segment_path_from_shapes(shapes)
}

#[cfg(test)]
mod test {
    use oxvg_parse::Parse;

    use crate::{optimize::Tolerance, paths::segment, Path};

    #[test]
    fn unite_squares_aligned_winding() {
        let background = Path::parse_string("M0,0 L0,10 L10,10 L10,0 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(&output.to_string(), "M0 0h10v5h5v10H5v-5H0V0Z");
    }

    #[test]
    fn unite_squares_opposite_winding() {
        let background = Path::parse_string("M0,0 L10,0 L10,10 L0,10 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(output.to_string(), "M0 0h10v5h5v10H5v-5H0V0Z");
    }

    #[test]
    fn unite_curves_aligned_winding() {
        let background = Path::parse_string("m10 50Q25 25 40 50T50 90 T 10 50").unwrap();
        let foreground = Path::parse_string("m20 60Q35 35 50 60T60 100 T 20 60").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(output.to_string(), "M10 50q14.5-24.167 29-1.611Q44.5 50.833 50 60q15 25 10 40-3.125 9.375-17.969-8.594Q31.875 84.375 10 50Z");
    }

    #[test]
    fn unite_curves_opposite_winding() {
        let background = Path::parse_string("m10 50Q25 25 40 50T50 90 T 10 50").unwrap();
        let foreground = Path::parse_string("m60 70Q40 20 10 70T60 70").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        // TODO: Update expected
        assert_eq!(output.to_string(), "M1.818 89.867Q1.797 83.672 10 70q3.344-5.574 6.564-9.904Q13.426 55.383 10 50q13.321-22.201 26.642-4.971Q49.658 44.144 60 70q-4.861 3.038-9.316 5.707Q52.091 83.728 50 90q-2.815 8.444-15.135-5.294-33.004 17.563-33.046 5.161Z");
    }

    #[test]
    #[ignore = "failing"]
    fn unite_arc_aligned_winding() {
        let background = Path::parse_string("M10 5a5 5 0 1 0 -10 0a5 5 0 1 0 10 0").unwrap();
        let foreground = Path::parse_string("M10 10a5 5 0 1 0 -10 0a5 5 0 1 0 10 0").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        // TODO: Update expected
        assert_eq!(output.to_string(), "m0 5A5 5 0 0 1 10 5a5 5 0 0 1-.67 2.5A5 5 0 0 1 10 10a5 5 0 0 1-10 0a5 5 0 0 1 .67-2.5A5 5 0 0 1 0 5");
    }

    #[test]
    #[ignore = "failing"]
    fn unite_arc_opposite_winding() {
        let background = Path::parse_string("M0 5a5 5 0 1 0 10 0a5 5 0 1 0 -10 0").unwrap();
        let foreground = Path::parse_string("M10 10a5 5 0 1 0 -10 0a5 5 0 1 0 10 0").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(output.to_string(), "m0 5A5 5 0 0 1 10 5a5 5 0 0 1-.67 2.5A5 5 0 0 1 10 10a5 5 0 0 1-10 0a5 5 0 0 1 .67-2.5A5 5 0 0 1 0 5");
    }

    #[test]
    #[ignore = "failing"]
    fn unite_various_aligned_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground =
            Path::parse_string("M5 10L10 0C20 10 10 10 13 13a5 5 0 0 1 -5 -5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(output.to_string(), "m5 5h2.5L10 0c2.05 2.05 3.26 3.68 3.91 5L15 5c2.71 2.71 1.01 3.95-1.11 4.52c-.91 1.36-2.26 2.11-.89 3.48a5 5 0 0 1-4.7-3.3a5 5 0 0 1-1.53-.88L5 10l1-2a5 5 0 0 1-1-3");
    }

    #[test]
    #[ignore = "failing"]
    fn unite_various_opposite_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground = Path::parse_string("M10 10C15 10 10 5 15 7.5a5 5 0 1 0 -5 0h-5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background.union(&foreground).to_svg(tolerance, false);
        assert_eq!(output.to_string(), "m5 5h2.85a5 5 0 1 1 8.33 1.56c1.45 3.08-4.65 3.41-5.94 3.44q-.11 0-.24 0a5 5 0 0 1-4-2l-1-.5l.67 0A5 5 0 0 1 5 5");
    }

    #[test]
    #[ignore = "failing"]
    fn intersect_various_aligned_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground =
            Path::parse_string("M5 10L10 0C20 10 10 10 13 13a5 5 0 0 1 -5 -5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .intersection(&foreground)
            .to_svg(tolerance, false);
        assert_eq!(output.to_string(), "m6 8l1.5-3h6.41c1.08 2.18.64 3.52-.03 4.52C12.09 10 10 10 10 10a5 5 0 0 1-1.7-.3A5 5 0 0 1 8 8l-1.23.82A5 5 0 0 1 6 8");
    }

    #[test]
    #[ignore = "failing"]
    fn intersect_various_opposite_winding() {
        let background = Path::parse_string("M5 5H15C20 10 10 10 10 10a5 5 0 0 1 -5 -5").unwrap();
        let foreground = Path::parse_string("M10 10C15 10 10 5 15 7.5a5 5 0 1 0 -5 0h-5Z").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .intersection(&foreground)
            .to_svg(tolerance, false);
        // TODO: Update expected
        assert_eq!(output.to_string(), "m5.67 7.5l4.33 0A5 5 0 0 1 7.85 5L15 5c.58.58.96 1.1 1.18 1.56A5 5 0 0 1 15 7.5c-4.92-2.46-.16 2.34-4.76 2.5C10.08 10 10 10 10 10L6 8a5 5 0 0 1-.33-.5");
    }
}
