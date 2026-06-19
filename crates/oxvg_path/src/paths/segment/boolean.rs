use crate::paths::{
    events,
    segment::{
        boolean::{
            contour::{connect_edges, Contour},
            event_queue::EventQueue,
        },
        Path, Tolerance, ToleranceSquared,
    },
};

mod contour;
pub(crate) mod event_queue;
mod splay;
pub(crate) mod sweep_event;
pub(crate) mod utils;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Operation {
    Intersection,
    Difference,
    Union,
    Xor,
}

impl Path {
    /// Runs an OR boolean operation against each shape in the path. Creates a shape equivalent
    /// to when `fill-rule: nonzero` is set.
    #[must_use]
    pub fn non_zero(mut self, tolerance: &Tolerance) -> Path {
        let foreground = Self(self.0.drain(0..self.0.len() / 2).collect());
        self.unite(&foreground, tolerance)
    }

    /// Runs an XOR boolean operation against each shape in the path. Creates a shape equivalent
    /// to when `fill-rule: evenodd` is set.
    #[must_use]
    pub fn even_odd(self, _: &Tolerance) -> Path {
        todo!("evenodd")
    }

    /// Runs an OR boolean operation against a background (self) and foreground (other) path.
    /// This generates a path where the areas covered by both the paths are joined.
    #[must_use]
    pub fn unite(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Union, other, tolerance)
    }

    /// Runs an AND boolean operation against a background (self) and foreground (other) path.
    /// This generates a path where only the areas covered by both the paths are retained.
    #[must_use]
    pub fn intersect(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Intersection, other, tolerance)
    }

    /// Runs an subtractive boolean operation against a background (self) and foreground (other) path.
    /// This generates a where where the areas covered only by the background are retained.
    #[must_use]
    pub fn difference(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Difference, other, tolerance);
        todo!("Difference cannot handle holes")
    }

    /// Runs an XOR boolean operation against a background (self) and foreground (other) path.
    /// This generates a path where only the areas covered by a single path are retained.
    #[must_use]
    pub fn xor(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Xor, other, tolerance);
        todo!("XOR cannot handle holes")
    }

    /// Runs a boolean path operation against a background (self) and foreground (other) path
    #[must_use]
    pub fn boolean(&self, operation: Operation, other: &Self, tolerance: &Tolerance) -> Path {
        let tolerance_squared = &tolerance.square();
        // Modified Martinez-Rueda algorithm
        // 1. Flatten paths to a polygonal representation
        let background = events::Path::from_segments(self, tolerance_squared);
        let foreground = events::Path::from_segments(other, tolerance_squared);

        // 2. Operate MR Algorithm against the polygon.
        background
            .boolean(operation, &foreground, tolerance_squared)
            .unwrap_or_else(|| trivial_result(self, other, operation))
    }
}

impl events::Path {
    /// Runs a boolean path operation against a background (self) and foreground (other) path.
    /// Returns it as a segment `[Path]`.
    pub fn boolean(
        &self,
        operation: Operation,
        foreground: &Self,
        tolerance: &ToleranceSquared,
    ) -> Option<Path> {
        let contours = self.contours(operation, foreground)?;

        // 3. Apply polygon operations to the path by cutting up `LineTo`, `CurveTo`, and `ArcTo` commands.
        Some(Path(
            contours
                .into_iter()
                .filter_map(|c| c.slice(self, foreground, tolerance))
                .collect(),
        ))
    }

    pub(crate) fn contours(&self, operation: Operation, foreground: &Self) -> Option<Vec<Contour>> {
        let mut event_queue = EventQueue::fill(self, foreground, operation);
        if event_queue.is_trivial() {
            return None;
        }

        let sorted_events = event_queue.subdivide();
        Some(connect_edges(sorted_events))
    }
}

fn trivial_result(background: &Path, foreground: &Path, operation: Operation) -> Path {
    match operation {
        Operation::Intersection => Path(vec![]),
        Operation::Difference => background.clone(),
        Operation::Union | Operation::Xor => {
            let mut result = background.clone();
            result.0.extend(foreground.0.iter().cloned());
            result
        }
    }
}

#[cfg(test)]
mod test {
    use oxvg_parse::Parse;

    use super::*;
    use crate::{paths::segment, Path};

    #[test]
    fn unite_squares_aligned_winding() {
        let background = Path::parse_string("M0,0 L0,10 L10,10 L10,0 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
        assert_eq!(&output.to_string(), "M0 0h10v5h5v10H5v-5H0V0Z");
    }

    #[test]
    fn unite_squares_opposite_winding() {
        let background = Path::parse_string("M0,0 L10,0 L10,10 L0,10 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
        assert_eq!(output.to_string(), "M0 0h10v5h5v10H5v-5H0V0Z");
    }

    #[test]
    fn unite_curves_aligned_winding() {
        let background = Path::parse_string("m10 50Q25 25 40 50T50 90 T 10 50").unwrap();
        let foreground = Path::parse_string("m20 60Q35 35 50 60T60 100 T 20 60").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
        assert_eq!(output.to_string(), "M10 50q14.5-24.167 29-1.611Q44.5 50.833 50 60q15 25 10 40-3.125 9.375-17.969-8.594Q31.875 84.375 10 50Z");
    }

    #[test]
    fn unite_curves_opposite_winding() {
        let background = Path::parse_string("m10 50Q25 25 40 50T50 90 T 10 50").unwrap();
        let foreground = Path::parse_string("m60 70Q40 20 10 70T60 70").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
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

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
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

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
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

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
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

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
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
            .intersect(&foreground, &Tolerance::default())
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
            .intersect(&foreground, &Tolerance::default())
            .to_svg(tolerance, false);
        // TODO: Update expected
        assert_eq!(output.to_string(), "m5.67 7.5l4.33 0A5 5 0 0 1 7.85 5L15 5c.58.58.96 1.1 1.18 1.56A5 5 0 0 1 15 7.5c-4.92-2.46-.16 2.34-4.76 2.5C10.08 10 10 10 10 10L6 8a5 5 0 0 1-.33-.5");
    }
}
