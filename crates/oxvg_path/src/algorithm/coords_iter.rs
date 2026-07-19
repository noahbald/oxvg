//! Types and methods for iterating [`geo::Coord`] coords of shapes.
use geo::{Coord, CoordsIter};

use crate::geometry::{Arc, CurveWithStart, Tolerance, ToleranceSquared};

#[derive(Clone)]
/// Iterates along the coordinates of a curve by stepping along the curve.
pub struct CurveCoordsIter<'a> {
    t: f64,
    t_step: f64,
    curve: &'a CurveWithStart,
}

#[derive(Clone)]
/// Iterates along the coordinates of a arc by stepping along the arc.
pub struct ArcCoordsIter<'a> {
    t: f64,
    t_step: f64,
    arc: &'a Arc,
}

impl<'a> CurveCoordsIter<'a> {
    /// Creates an iterator to step along a curve.
    pub fn new(curve: &'a CurveWithStart) -> Self {
        Self::new_with_tolerance(curve, Tolerance::default().square())
    }

    /// Creates an iterator to step along a curve, with each step based on the given tolerance.
    pub fn new_with_tolerance(
        curve: &'a CurveWithStart,
        tolerance_squared: ToleranceSquared,
    ) -> Self {
        // TODO: PERF: Adaptive steps would improve iteration count for consumers
        let mut t_step = 1.0;
        for _ in 0..7 {
            let distance_squared = curve.start.distance_squared(curve.point_at(t_step));
            if distance_squared < *tolerance_squared {
                break;
            }
            t_step /= 2.0;
        }
        t_step = t_step.max(f64::EPSILON);

        Self {
            t: 0.0,
            t_step,
            curve,
        }
    }
}

impl<'a> ArcCoordsIter<'a> {
    /// Creates an iterator to step along a arc.
    pub fn new(arc: &'a Arc) -> Self {
        let tolerance = Tolerance::default();
        Self::new_with_tolerance(arc, &tolerance, Tolerance::default().square())
    }

    /// Creates an iterator to step along a arc, with each step based on the given tolerance.
    pub fn new_with_tolerance(
        arc: &'a Arc,
        tolerance: &Tolerance,
        tolerance_squared: ToleranceSquared,
    ) -> Self {
        // TODO: PERF: Adaptive steps would improve iteration count for consumers
        let mut t_step = 1.0;
        for _ in 0..7 {
            if (t_step * arc.sweep_angle()).abs() <= tolerance.angular
                || arc
                    .start_point()
                    .distance_squared(arc.point_at_angle(t_step))
                    <= *tolerance_squared
            {
                break;
            }
            t_step /= 2.0;
        }
        t_step = t_step.max(f64::EPSILON);

        Self {
            t: 0.0,
            t_step,
            arc,
        }
    }
}

impl Iterator for CurveCoordsIter<'_> {
    type Item = Coord<f64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.t > 1.0 {
            return None;
        }
        let item = if self.t == 1.0 || self.t + self.t_step > 1.0 {
            self.curve.curve.end_point
        } else {
            self.curve.point_at(self.t)
        };
        self.t += self.t_step;
        Some(*item)
    }
}
impl ExactSizeIterator for CurveCoordsIter<'_> {
    fn len(&self) -> usize {
        len(self.t, self.t_step)
    }
}

impl Iterator for ArcCoordsIter<'_> {
    type Item = Coord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.t > 1.0 {
            return None;
        }
        let item = if self.t == 1.0 || self.t + self.t_step > 1.0 {
            self.arc.end_point()
        } else {
            self.arc.point_at(self.t)
        };
        self.t += self.t_step;
        Some(*item)
    }
}
impl ExactSizeIterator for ArcCoordsIter<'_> {
    fn len(&self) -> usize {
        len(self.t, self.t_step)
    }
}

#[allow(clippy::cast_sign_loss)]
fn len(t: f64, t_step: f64) -> usize {
    let total = t_step.powi(-1);
    let remaining = total - (t / t_step);

    // NOTE: Truncated
    remaining as usize
}

impl CoordsIter for CurveWithStart {
    type Iter<'a> = CurveCoordsIter<'a>;
    type ExteriorIter<'a> = CurveCoordsIter<'a>;
    type Scalar = f64;

    fn coords_iter(&self) -> Self::Iter<'_> {
        CurveCoordsIter::new(self)
    }

    fn coords_count(&self) -> usize {
        CurveCoordsIter::new(self).len()
    }

    fn exterior_coords_iter(&self) -> Self::ExteriorIter<'_> {
        self.coords_iter()
    }
}

impl CoordsIter for Arc {
    type Iter<'a> = ArcCoordsIter<'a>;
    type ExteriorIter<'a> = ArcCoordsIter<'a>;
    type Scalar = f64;

    fn coords_iter(&self) -> Self::Iter<'_> {
        ArcCoordsIter::new(self)
    }

    fn coords_count(&self) -> usize {
        ArcCoordsIter::new(self).len()
    }

    fn exterior_coords_iter(&self) -> Self::ExteriorIter<'_> {
        self.coords_iter()
    }
}
