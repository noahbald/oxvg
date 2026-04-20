use crate::paths::{
    events,
    segment::{
        boolean::{contour::connect_edges, event_queue::EventQueue},
        Path, Tolerance, ToleranceSquared,
    },
};

mod contour;
mod event_queue;
mod splay;
mod sweep_event;
mod utils;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    Union,
    Intersection,
    Difference,
    Xor,
}

impl Path {
    /// Runs an OR boolean operation against each shape in the path. Creates a shape equivalent
    /// to when `fill-rule: nonzero` is set.
    pub fn non_zero(mut self, tolerance: &Tolerance) -> Path {
        let foreground = Self(self.0.drain(0..self.0.len() / 2).collect());
        self.unite(&foreground, tolerance)
    }

    /// Runs an OR boolean operation against a background (self) and foreground (other) path.
    /// This generates a path where the areas covered by both the paths are joined.
    pub fn unite(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Union, other, tolerance)
    }

    /// Runs an AND boolean operation against a background (self) and foreground (other) path.
    /// This generates a path where only the areas covered by both the paths are retained.
    pub fn intersect(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Intersection, other, tolerance)
    }

    /// Runs an subtractive boolean operation against a background (self) and foreground (other) path.
    /// This generates a where where the areas covered only by the background are retained.
    pub fn difference(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Difference, other, tolerance)
    }

    /// Runs an XOR boolean operation against a background (self) and foreground (other) path.
    /// This generates a path where only the areas covered by a single path are retained.
    pub fn xor(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Xor, other, tolerance)
    }

    /// Runs a boolean path operation against a background (self) and foreground (other) path
    pub fn boolean(&self, operation: Operation, other: &Self, tolerance: &Tolerance) -> Path {
        let tolerance_squared = &tolerance.square();
        // Modified Martinez-Rueda algorithm
        // 1. Flatten paths to a polygonal representation
        let background = events::Path::from_segments(self, tolerance_squared);
        let foreground = events::Path::from_segments(other, tolerance_squared);

        // 2. Operate MR Algorithm against the polygon.
        background
            .boolean(operation, &foreground, tolerance_squared)
            .unwrap_or_else(|| {
                return trivial_result(self, other, operation);
            })
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
        let mut event_queue = EventQueue::fill(self, foreground, operation);
        if event_queue.is_trivial() {
            return None;
        }

        let sorted_events = event_queue.subdivide();
        let contours = connect_edges(sorted_events);

        // 3. Apply polygon operations to the path by cutting up `LineTo`, `CurveTo`, and `ArcTo` commands.
        Some(Path(
            contours
                .into_iter()
                .filter_map(|c| c.slice(self, foreground, tolerance))
                .collect(),
        ))
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
            .to_svg(tolerance);
        assert_eq!(&output.to_string(), "M0 0h10V0V5h5V15H5V10H0V0");
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
            .to_svg(tolerance);
        assert_eq!(output.to_string(), "M0 0h10V0V5h5V15H5V10H0V0");
    }
}
