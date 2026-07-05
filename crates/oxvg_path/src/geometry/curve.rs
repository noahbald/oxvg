//! Types for representing bezier curves.
use crate::geometry::{line::Intersection, Line, Point, Tolerance, ToleranceSquared};

#[derive(Debug, Clone, Copy, PartialEq)]
/// A cubic bezier curve.
pub struct Curve {
    /// The start control of the curve.
    pub start_control: Point,
    /// The end control of the curve.
    pub end_control: Point,
    /// The end point.
    pub end_point: Point,
}

/// A cubic bezier curve from a start point.
///
/// Useful as a shorthand for [`Curve`] methods that would usually require a start point
/// as an argument.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveWithStart {
    /// The start point of the curve.
    pub start: Point,
    /// The bezier curve.
    pub curve: Curve,
}

/// A cubic bezier curve.
pub type CubicBezierTo = Curve;

#[derive(Debug)]
/// A smooth cubic bezier curve.
pub struct SmoothBezierTo {
    /// The end control of the curve.
    pub end_control: Point,
    /// The end point.
    pub end_point: Point,
}

#[derive(Debug)]
/// A quadratic bezier curve.
pub struct QuadraticBezierTo {
    /// The start/end control of the curve.
    pub quad_control: Point,
    /// The end point.
    pub end_point: Point,
}

#[derive(Debug)]
/// A smooth quadratic bezier curve.
pub struct SmoothQuadraticBezierTo {
    /// The end point.
    pub end_point: Point,
}

impl From<[f64; 6]> for Curve {
    fn from(value: [f64; 6]) -> Self {
        Self {
            start_control: Point::new(value[0], value[1]),
            end_control: Point::new(value[2], value[3]),
            end_point: Point::new(value[4], value[5]),
        }
    }
}

impl Curve {
    /// Returns a new curve.
    pub fn new(start_control: Point, end_control: Point, end_point: Point) -> Self {
        Self {
            start_control,
            end_control,
            end_point,
        }
    }

    /// Returns a new curve based on the given quadratic control.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new_quad(Point::Y * 3.0, Point::ZERO, Point::X * 3.0),
    ///   Curve::new(Point::new(0.0, 2.0), Point::new(1.0, 2.0), Point::X * 3.0),
    /// );
    /// ```
    pub fn new_quad(quad_control: Point, start_point: Point, end_point: Point) -> Self {
        let cp1 = start_point + (quad_control - start_point) * (2.0 / 3.0);
        let cp2 = end_point + (quad_control - end_point) * (2.0 / 3.0);
        Curve::new(cp1, cp2, end_point)
    }

    /// Returns a bezier curve with an associated start point.
    pub fn with_start(self, start: Point) -> CurveWithStart {
        CurveWithStart { start, curve: self }
    }

    /// Returns the quad control of the curve, if the curve is quadratic.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::new(0.0, 2.0), Point::new(1.0, 2.0), Point::X * 3.0)
    ///     .quad_control(Point::ZERO, Tolerance::default().square()),
    ///   Some(Point::Y * 3.0),
    /// );
    /// ```
    pub fn quad_control(&self, start: Point, tolerance: ToleranceSquared) -> Option<Point> {
        let quad = self.quad_control_unchecked(start);
        if quad.distance_squared(self.end_point + (self.end_control - self.end_point) * 1.5)
            < *tolerance
        {
            Some(quad)
        } else {
            None
        }
    }

    /// Returns the quad control of the curve, without checking if the curve is quadratic.
    ///
    /// # Correctness
    ///
    /// If the source curve is not quadratic, then the returned
    /// quad control is not on the same curve.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::new(0.0, 2.0), Point::new(1.0, 2.0), Point::X * 3.0)
    ///     .quad_control_unchecked(Point::ZERO),
    ///   Point::Y * 3.0,
    /// );
    /// ```
    pub fn quad_control_unchecked(&self, start: Point) -> Point {
        start + (self.start_control - start) * 1.5
    }

    /// Returns the cubic bezier form of the curve.
    #[must_use]
    pub fn cubic_bezier(&self) -> CubicBezierTo {
        *self
    }

    /// Returns the smooth cubic bezier form of the curve, if possible.
    pub fn smooth_bezier(
        &self,
        start: Point,
        control: Option<Point>,
        tolerance: ToleranceSquared,
    ) -> Option<SmoothBezierTo> {
        if self.is_smooth(start, control, tolerance) {
            Some(self.smooth_bezier_unchecked())
        } else {
            None
        }
    }

    /// Returns the smooth cubic bezier form of the curve, without checking if
    /// it's smooth.
    ///
    /// # Correctness
    ///
    /// If the source curve is not smooth, then the returned
    /// smooth cubic bezier is not the same curve.
    pub fn smooth_bezier_unchecked(&self) -> SmoothBezierTo {
        SmoothBezierTo {
            end_control: self.end_control,
            end_point: self.end_point,
        }
    }

    /// Returns the smooth quadratic bezier form of the curve, if possible
    pub fn quadratic_bezier(
        &self,
        start: Point,
        tolerance: ToleranceSquared,
    ) -> Option<QuadraticBezierTo> {
        self.quad_control(start, tolerance)
            .map(|quad_control| self.quadratic_bezier_unchecked(quad_control))
    }

    /// Returns the smooth quadratic bezier form of the curve, without
    /// checking whether the quad control is correct.
    ///
    /// # Correctness
    ///
    /// If the quad control is not corect, then the returned
    /// smooth quadratic bezier is not the same curve.
    pub fn quadratic_bezier_unchecked(&self, quad_control: Point) -> QuadraticBezierTo {
        QuadraticBezierTo {
            quad_control,
            end_point: self.end_point,
        }
    }

    /// Returns the smooth quadratic bezier form of the curve, without
    /// checking whether the source curve is smooth with some quad control.
    ///
    /// # Correctness
    ///
    /// If the source curve is not a smooth quadratic bezier, then the returned
    /// smooth quadratic bezier is not the same curve.
    pub fn smooth_quadratic_bezier_unchecked(&self) -> SmoothQuadraticBezierTo {
        SmoothQuadraticBezierTo {
            end_point: self.end_point,
        }
    }

    /// Returns the smooth quadratic bezier form of the curve, without
    /// checking whether the quad control is correct.
    ///
    /// # Correctness
    ///
    /// If the quad control is not correct, then the returned
    /// smooth quadratic bezier is not the same curve.
    pub fn smooth_quadratic_bezier_unchecked_quad(
        &self,
        start: Point,
        control: Option<Point>,
        quad_control: Point,
        tolerance: ToleranceSquared,
    ) -> Option<SmoothQuadraticBezierTo> {
        if control.is_some_and(|cp| quad_control.distance_squared(cp.reflect(start)) < *tolerance) {
            Some(self.smooth_quadratic_bezier_unchecked())
        } else {
            None
        }
    }

    /// Returns the smooth quadratic bezier form of the curve, if possible.
    pub fn smooth_quadratic_bezier(
        &self,
        start: Point,
        control: Option<Point>,
        tolerance: ToleranceSquared,
    ) -> Option<SmoothQuadraticBezierTo> {
        self.smooth_quadratic_bezier_unchecked_quad(
            start,
            control,
            self.quad_control(start, tolerance)?,
            tolerance,
        )
    }

    /// Returns whether a curve is convex
    ///
    /// A curve is convex when the middle of the curve's line is below the curve's midpoint
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert!(Curve::new(Point::X, Point::X, Point::UNIT).is_convex());
    /// ```
    pub fn is_convex(&self) -> bool {
        let end_control_line = Line::new(Point::new(0.0, 0.0), self.end_control);
        let start_control_line = Line::new(self.start_control, self.end_point);
        let Some(Intersection::SinglePoint {
            intersection: center,
            ..
        }) = end_control_line.intersection(&start_control_line)
        else {
            return false;
        };
        let center = Point(center);
        (self.end_control.x < center.x) == (center.x < 0.0)
            && (self.end_control.y < center.y) == (center.y < 0.0)
            && (self.end_point.x < center.x) == (center.x < self.start_control.x)
            && (self.end_point.y < center.y) == (center.y < self.start_control.y)
    }

    /// Returns whether the arc fits on a straight line.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert!(Curve::new(Point::UNIT, Point::UNIT * 2.0, Point::UNIT * 3.0)
    ///   .is_straight(Point::ZERO, &Tolerance::default()));
    /// ```
    pub fn is_straight(&self, start: Point, tolerance: &Tolerance) -> bool {
        let chord = start.distance(self.end_point);
        if chord < tolerance.positional {
            return true;
        }
        let tolerance_scaled = tolerance.positional * chord;
        start.cross(self.end_point, self.start_control).abs() < tolerance_scaled
            && start.cross(self.end_point, self.end_control).abs() < tolerance_scaled
    }

    /// Returns various bezier forms of the curve.
    pub fn types(
        &self,
        start: Point,
        control: Option<Point>,
        tolerance: ToleranceSquared,
    ) -> (
        CubicBezierTo,
        Option<SmoothBezierTo>,
        Option<QuadraticBezierTo>,
        Option<SmoothQuadraticBezierTo>,
    ) {
        let cubic_bezier = self.cubic_bezier();
        let smooth_bezier = self.smooth_bezier(start, control, tolerance);
        let (quadratic_bezier, smooth_quadratic_bezier) =
            if let Some(quad_control) = self.quad_control(start, tolerance) {
                (
                    Some(self.quadratic_bezier_unchecked(quad_control)),
                    self.smooth_quadratic_bezier_unchecked_quad(
                        start,
                        control,
                        quad_control,
                        tolerance,
                    ),
                )
            } else {
                (None, None)
            };
        (
            cubic_bezier,
            smooth_bezier,
            quadratic_bezier,
            smooth_quadratic_bezier,
        )
    }

    /// Returns whether the curve is representable as a smooth bezier curve.
    pub fn is_smooth(
        &self,
        start: Point,
        control: Option<Point>,
        tolerance: ToleranceSquared,
    ) -> bool {
        self.start_control
            .distance_squared(control.map_or(start, |c| c.reflect(start)))
            < *tolerance
    }

    /// Returns the distance of the start and end control points.
    pub fn control_point_distance_squared(&self, start: Point) -> (f64, f64) {
        let end = self.end_point;
        (
            control_point_distance_squared(self.start_control, start, end),
            control_point_distance_squared(self.end_control, start, end),
        )
    }

    /// Divides the curve into two halves drawn from some start point. Returns
    /// the left half and the right half with their starting points.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).subdivide(Point::ZERO),
    ///   (
    ///     Curve::new(Point::X * 0.5, Point::X * 0.75, Point::new(0.875, 0.125)),
    ///     Point::new(0.875, 0.125),
    ///     Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT),
    ///   )
    /// );
    /// ```
    pub fn subdivide(&self, start: Point) -> (Curve, Point, Curve) {
        self.subdivide_t(start, 0.5)
    }

    /// Returns two halves of the curve by a point that lies on the curve, up to some tolerance.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).subdivide_at(
    ///     Point::ZERO,
    ///     Point::new(0.875, 0.125),
    ///     Tolerance::default().square(),
    ///   ),
    ///   Some((
    ///     Curve::new(Point::X * 0.5, Point::X * 0.75, Point::new(0.875, 0.125)),
    ///     Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT),
    ///   )),
    /// );
    /// ```
    pub fn subdivide_at(
        &self,
        start: Point,
        at: Point,
        tolerance: ToleranceSquared,
    ) -> Option<(Curve, Curve)> {
        let result = self.subdivide_t(start, self.t_at(start, at, tolerance)?);
        Some((result.0, result.2))
    }

    /// Returns two divisions of the curve by the percentage along the curve, as a number
    /// between `0.0` and `1.0`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).subdivide_t(
    ///     Point::ZERO,
    ///     0.5,
    ///   ),
    ///   (
    ///     Curve::new(Point::X * 0.5, Point::X * 0.75, Point::new(0.875, 0.125)),
    ///     Point::new(0.875, 0.125),
    ///     Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT),
    ///   )
    /// );
    /// ```
    #[allow(clippy::similar_names)]
    pub fn subdivide_t(&self, start: Point, t: f64) -> (Curve, Point, Curve) {
        let p0 = start;
        let p1 = self.start_control;
        let p2 = self.end_control;
        let p3 = self.end_point;

        let p01 = p0.lerp(p1, t);
        let p12 = p1.lerp(p2, t);
        let p23 = p2.lerp(p3, t);
        let p012 = p01.lerp(p12, t);
        let p123 = p12.lerp(p23, t);
        let p0123 = p012.lerp(p123, t);

        let left = Curve::new(p01, p012, p0123);
        let right = Curve::new(p123, p23, p3);
        (left, p0123, right)
    }

    /// Returns the point `t` percent along a curve's chord from some
    /// start point, where `1.0` is `100%`
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).point_at_from(
    ///     Point::ZERO,
    ///     0.5,
    ///   ),
    ///   Point::new(0.875, 0.125),
    /// );
    /// ```
    pub fn point_at_from(&self, start: Point, t: f64) -> Point {
        let start_control = self.start_control;
        let end_control = self.end_control;
        let end_point = self.end_point;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        start * mt3 + start_control * 3.0 * mt2 * t + end_control * 3.0 * mt * t2 + end_point * t3
    }

    /// Returns the percent along the curve a point lies, to some tolerance.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).t_at(
    ///     Point::ZERO,
    ///     Point::new(0.875, 0.125),
    ///     Tolerance::default().square(),
    ///   ),
    ///   Some(0.5),
    /// );
    /// ```
    #[allow(clippy::similar_names)]
    pub fn t_at(&self, start: Point, at: Point, tolerance: ToleranceSquared) -> Option<f64> {
        const MAX_ITER: usize = 8;
        let p0 = start;
        let p1 = self.start_control;
        let p2 = self.end_control;
        let p3 = self.end_point;

        let da = (p3 - p0) + (p1 - p2) * 3.0;
        let db = (p0 - p1 * 2.0 + p2) * 2.0;
        let dc = p1 - p0;

        let b_prime = |t: f64| -> Point {
            let t2 = t * t;
            (da * t2 + db * t + dc) * 3.0
        };
        let bd_prime = |t: f64| -> Point { (da * (2.0 * t) + db) * 3.0 };

        let tol_sq = *tolerance;

        // Try several seeds spread across [0,1] and keep the best converged result.
        let seeds: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
        let mut best_t: Option<f64> = None;
        let mut best_dist_sq = f64::INFINITY;

        for &seed in &seeds {
            let mut t = seed;

            for _ in 0..MAX_ITER {
                let bt = self.point_at_from(start, t);
                let diff = bt - at;

                let bp = b_prime(t);
                let bp2 = bd_prime(t);

                let f1 = diff.dot(bp);
                let f2 = bp.dot(bp) + diff.dot(bp2);

                if f2.abs() < 1e-12 {
                    break;
                }

                let step = f1 / f2;
                t -= step;
                t = t.clamp(0.0, 1.0);

                if step.abs() < 1e-7 {
                    break;
                }
            }

            let dist_sq = self.point_at_from(start, t).distance_squared(at);
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_t = Some(t);
            }
        }

        if best_dist_sq <= tol_sq {
            best_t
        } else {
            None
        }
    }

    /// Returns an equivalent curve spanning from the end point to the start.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).reverse(Point::ZERO),
    ///   Curve::new(Point::X, Point::X, Point::ZERO),
    /// );
    /// ```
    #[must_use]
    pub fn reverse(&self, start: Point) -> Self {
        Curve::new(self.end_control, self.start_control, start)
    }

    /// Creates a subcurve between the percentage `t1` and `t2`, as numbers between `0.0` and `1.0`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).clamp_t(
    ///     Point::ZERO,
    ///     0.5,
    ///     1.0
    ///   ),
    ///   Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT),
    /// );
    /// ```
    #[must_use]
    pub fn clamp_t(&self, start: Point, t1: f64, t2: f64) -> Self {
        debug_assert!((0.0..=1.0).contains(&t1));
        debug_assert!((0.0..=1.0).contains(&t2));
        debug_assert!(t1 <= t2);
        let (_, start, right) = self.subdivide_t(start, t1);
        let t2 = if (1.0 - t1).abs() < 1e-10 {
            1.0
        } else {
            (t2 - t1) / (1.0 - t1)
        };
        right.subdivide_t(start, t2).0
    }

    /// Creates a subcurve between the points `t1` and `t2`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT).clamp_at(
    ///     Point::ZERO,
    ///     Point::new(0.875, 0.125),
    ///     Point::UNIT,
    ///     Tolerance::default().square(),
    ///   ),
    ///   Some(Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT)),
    /// );
    /// ```
    #[must_use]
    pub fn clamp_at(
        &self,
        start: Point,
        t1: Point,
        t2: Point,
        tolerance: ToleranceSquared,
    ) -> Option<Self> {
        Some(self.clamp_t(
            start,
            self.t_at(start, t1, tolerance)?,
            self.t_at(start, t2, tolerance)?,
        ))
    }
}

fn control_point_distance_squared(control: Point, start: Point, end: Point) -> f64 {
    let vector = end - start;
    let dot = vector.dot(vector);
    if dot == 0.0 {
        return control.distance_squared(start);
    }

    let t = ((control.x - start.x) * vector.x + (control.y - start.y) * vector.y) / dot;
    let t = t.clamp(0.0, 1.0);
    let projection = Point::new(start.x + t * vector.x, start.y + t * vector.y);
    control.distance_squared(projection)
}

impl CurveWithStart {
    /// Returns the quad control of the curve, if the curve is quadratic.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::new(0.0, 2.0), Point::new(1.0, 2.0), Point::X * 3.0)
    ///     .with_start(Point::ZERO)
    ///     .quad_control(Tolerance::default().square()),
    ///   Some(Point::Y * 3.0),
    /// );
    /// ```
    pub fn quad_control(&self, tolerance: ToleranceSquared) -> Option<Point> {
        self.curve.quad_control(self.start, tolerance)
    }

    /// Returns the quad control of the curve, without checking if the curve is quadratic.
    ///
    /// # Correctness
    ///
    /// If the source curve is not quadratic, then the returned
    /// quad control is not on the same curve.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::new(0.0, 2.0), Point::new(1.0, 2.0), Point::X * 3.0)
    ///     .with_start(Point::ZERO)
    ///     .quad_control_unchecked(),
    ///   Point::Y * 3.0,
    /// );
    /// ```
    pub fn quad_control_unchecked(&self) -> Point {
        self.curve.quad_control_unchecked(self.start)
    }

    /// Returns whether the arc fits on a straight line.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert!(Curve::new(Point::UNIT, Point::UNIT * 2.0, Point::UNIT * 3.0)
    ///   .with_start(Point::ZERO)
    ///   .is_straight(&Tolerance::default()));
    /// ```
    pub fn is_straight(&self, tolerance: &Tolerance) -> bool {
        self.curve.is_straight(self.start, tolerance)
    }

    /// Returns the distance of the start and end control points.
    pub fn control_point_distance_squared(&self) -> (f64, f64) {
        self.curve.control_point_distance_squared(self.start)
    }

    /// Divides the curve into two halves drawn from some start point. Returns
    /// the left half and the right half with their starting points.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT)
    ///     .with_start(Point::ZERO)
    ///     .subdivide(),
    ///   (
    ///     Curve::new(Point::X * 0.5, Point::X * 0.75, Point::new(0.875, 0.125))
    ///       .with_start(Point::ZERO),
    ///     Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT)
    ///       .with_start(Point::new(0.875, 0.125)),
    ///   )
    /// );
    /// ```
    pub fn subdivide(&self) -> (Self, Self) {
        self.subdivide_t(0.5)
    }

    /// Returns two halves of the curve by a point that lies on the curve, up to some tolerance.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT)
    ///     .with_start(Point::ZERO)
    ///     .subdivide_at(Point::new(0.875, 0.125), Tolerance::default().square()),
    ///   Some((
    ///     Curve::new(Point::X * 0.5, Point::X * 0.75, Point::new(0.875, 0.125))
    ///       .with_start(Point::ZERO),
    ///     Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT)
    ///       .with_start(Point::new(0.875, 0.125)),
    ///   ))
    /// );
    /// ```
    pub fn subdivide_at(&self, at: Point, tolerance: ToleranceSquared) -> Option<(Self, Self)> {
        Some(self.subdivide_t(self.t_at(at, tolerance)?))
    }

    /// Returns two divisions of the curve by the percentage along the curve, as a number
    /// between `0.0` and `1.0`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT)
    ///     .with_start(Point::ZERO)
    ///     .subdivide_t(0.5),
    ///   (
    ///     Curve::new(Point::X * 0.5, Point::X * 0.75, Point::new(0.875, 0.125))
    ///       .with_start(Point::ZERO),
    ///     Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT)
    ///       .with_start(Point::new(0.875, 0.125)),
    ///   )
    /// );
    /// ```
    pub fn subdivide_t(&self, t: f64) -> (Self, Self) {
        let (left, middle, right) = self.curve.subdivide_t(self.start, t);
        (
            Self {
                start: self.start,
                curve: left,
            },
            Self {
                start: middle,
                curve: right,
            },
        )
    }

    /// Returns the point `t` percent along a curve's chord from some
    /// start point, where `1.0` is `100%`
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT)
    ///     .with_start(Point::ZERO)
    ///     .point_at(0.5),
    ///   Point::new(0.875, 0.125),
    /// );
    /// ```
    pub fn point_at(&self, t: f64) -> Point {
        self.curve.point_at_from(self.start, t)
    }

    #[allow(clippy::similar_names)]
    /// Returns the percent along the curve a point lies, to some tolerance.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point, Tolerance};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT)
    ///     .with_start(Point::ZERO)
    ///     .t_at(Point::new(0.875, 0.125), Tolerance::default().square()),
    ///   Some(0.5),
    /// );
    /// ```
    pub fn t_at(&self, at: Point, tolerance: ToleranceSquared) -> Option<f64> {
        self.curve.t_at(self.start, at, tolerance)
    }

    /// Returns an equivalent curve spanning from the end point to the start.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT)
    ///     .with_start(Point::ZERO)
    ///     .reverse(),
    ///   Curve::new(Point::X, Point::X, Point::ZERO)
    ///     .with_start(Point::UNIT),
    /// );
    /// ```
    #[must_use]
    pub fn reverse(&self) -> Self {
        Self {
            start: self.curve.end_point,
            curve: self.curve.reverse(self.start),
        }
    }

    /// Creates a subcurve between the percentage `t1` and `t2`, as numbers between `0.0` and `1.0`.
    ///
    /// # Example
    ///
    /// ```
    /// use oxvg_path::geometry::{Curve, Point};
    ///
    /// assert_eq!(
    ///   Curve::new(Point::X, Point::X, Point::UNIT)
    ///     .with_start(Point::ZERO)
    ///     .clamp_t(0.5, 1.0),
    ///   Curve::new(Point::new(1.0, 0.25), Point::new(1.0, 0.5), Point::UNIT)
    ///     .with_start(Point::new(0.875, 0.125)),
    /// );
    /// ```
    #[must_use]
    pub fn clamp_t(&self, t1: f64, t2: f64) -> Self {
        Self {
            start: self.point_at(t1),
            curve: self.curve.clamp_t(self.start, t1, t2),
        }
    }
}
