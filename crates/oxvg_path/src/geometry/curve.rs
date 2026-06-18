//! Types for representing bezier curves.
use crate::{
    command,
    geometry::{line::Intersection, Circle, ErrorOptions, Line, Point},
    optimize::Tolerance,
    paths::segment::ToleranceSquared,
    position::Position,
};

#[derive(Debug, Clone, Copy, PartialEq)]
/// A bezier curve.
pub struct Curve(
    /// The args of an SVG cubic bezier to (`C`) command.
    /// [MDN](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/d#cubic_b%C3%A9zier_curve)
    pub [f64; 6],
);

#[derive(Debug)]
/// A smooth cubic bezier curve.
pub struct CubicBezierTo {
    /// The start control of the curve.
    pub start_control: Point,
    /// The end control of the curve.
    pub end_control: Point,
    /// The end point.
    pub end_point: Point,
}

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

impl Curve {
    /// Returns a new curve.
    pub fn new(start_control: Point, end_control: Point, end_point: Point) -> Self {
        Self([
            start_control.x(),
            start_control.y(),
            end_control.x(),
            end_control.y(),
            end_point.x(),
            end_point.y(),
        ])
    }

    /// Returns the start control point.
    pub const fn start_control(&self) -> Point {
        Point([self.0[0], self.0[1]])
    }

    /// Returns the end control point.
    pub const fn end_control(&self) -> Point {
        Point([self.0[2], self.0[3]])
    }

    /// Returns the end point.
    pub const fn end_point(&self) -> Point {
        Point([self.0[4], self.0[5]])
    }

    /// Returns the quad control of the curve, if the curve is quadratic.
    pub fn quad_control(&self, start: Point, tolerance: &ToleranceSquared) -> Option<Point> {
        let quad = self.quad_control_unchecked(start);
        if quad
            .distance_squared(&(self.end_point() + (self.end_control() - self.end_point()) * 1.5))
            < **tolerance
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
    pub fn quad_control_unchecked(&self, start: Point) -> Point {
        start + (self.start_control() - start) * 1.5
    }

    /// Returns a curve based on a bezier commands
    pub fn smooth_bezier_by_args<'a>(prev: &'a Position, item: &'a Position) -> Option<Self> {
        match item.command {
            command::Data::SmoothBezierBy(s) => {
                let p_data = prev.command.args();
                let len = p_data.len();
                if len < 4 {
                    return Some(Self([0.0, 0.0, s[0], s[1], s[2], s[3]]));
                }
                Some(Self([
                    p_data[len - 2] - p_data[len - 4],
                    p_data[len - 1] - p_data[len - 3],
                    s[0],
                    s[1],
                    s[2],
                    s[3],
                ]))
            }
            command::Data::CubicBezierBy(c) => Some(Self(c)),
            _ => None,
        }
    }

    /// Returns the cubic bezier form of the curve.
    pub fn cubic_bezier(&self) -> CubicBezierTo {
        CubicBezierTo {
            start_control: self.start_control(),
            end_control: self.end_control(),
            end_point: self.end_point(),
        }
    }

    /// Returns the smooth cubic bezier form of the curve, if possible.
    pub fn smooth_bezier(
        &self,
        start: Point,
        control: Option<Point>,
        tolerance: &ToleranceSquared,
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
            end_control: self.end_control(),
            end_point: self.end_point(),
        }
    }

    /// Returns the smooth quadratic bezier form of the curve, if possible
    pub fn quadratic_bezier(
        &self,
        start: Point,
        tolerance: &ToleranceSquared,
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
            end_point: self.end_point(),
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
            end_point: self.end_point(),
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
        tolerance: &ToleranceSquared,
    ) -> Option<SmoothQuadraticBezierTo> {
        if control.is_some_and(|cp| quad_control.distance_squared(&cp.reflect(start)) < **tolerance)
        {
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
        tolerance: &ToleranceSquared,
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
    pub fn is_convex(&self) -> bool {
        let end_control_line = Line([Point([0.0, 0.0]), self.end_control()]);
        let start_control_line = Line([self.start_control(), self.end_point()]);
        let Intersection::Intersection(center) = end_control_line.intersection(&start_control_line)
        else {
            return false;
        };
        (self.end_control().x() < center.x()) == (center.x() < 0.0)
            && (self.end_control().y() < center.y()) == (center.y() < 0.0)
            && (self.end_point().x() < center.x()) == (center.x() < self.start_control().x())
            && (self.end_point().y() < center.y()) == (center.y() < self.start_control().y())
    }

    /// Returns whether a curve is an arc of a circle
    #[deprecated]
    pub fn is_arc(&self, circle: &Circle, make_arcs: &ErrorOptions, error: f64) -> bool {
        let tolerance = f64::min(
            make_arcs.threshold * error,
            (make_arcs.tolerance * circle.radius) / 100.0,
        );
        [0.0, 0.25, 0.5, 0.75, 1.0]
            .into_iter()
            .all(|t| (self.point_at(t).distance(&circle.center) - circle.radius).abs() <= tolerance)
    }

    /// Returns whether a curve from a previous command is an arc of a circle
    #[deprecated]
    pub fn is_arc_prev(&self, circle: &Circle, make_arcs: &ErrorOptions, error: f64) -> bool {
        self.is_arc(
            &Circle {
                center: circle.center + self.end_point(),
                radius: circle.radius,
            },
            make_arcs,
            error,
        )
    }

    /// Returns whether the arc fits on a straight line.
    pub fn is_straight(&self, start: Point, tolerance: &Tolerance) -> bool {
        let chord = start.distance(&self.end_point());
        if chord < tolerance.positional {
            return true;
        }
        let tolerance_scaled = tolerance.positional * chord;
        Point::cross(start, self.end_point(), self.start_control()).abs() < tolerance_scaled
            && Point::cross(start, self.end_point(), self.end_control()).abs() < tolerance_scaled
    }

    /// Returns various bezier forms of the curve.
    pub fn types(
        &self,
        start: Point,
        control: Option<Point>,
        tolerance: &ToleranceSquared,
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
        tolerance: &ToleranceSquared,
    ) -> bool {
        self.start_control()
            .distance_squared(&control.map_or(start, |c| c.reflect(start)))
            < **tolerance
    }

    /// Returns whether the arc fits on a straight line
    #[deprecated = "For deprecated use on arc-by commands"]
    pub fn is_data_straight(args: &[f64], tolerance: f64) -> bool {
        // Get line equation a·x + b·y + c = 0 coefficients a, b (c = 0) by start and end points.
        let i = args.len() - 2;
        let a = -args[i + 1]; // y1 − y2 (y1 = 0)
        let b = args[i]; // x2 − x1 (x1 = 0)
        let d = 1.0 / (a * a + b * b); // same part for all points

        if i <= 1 || !d.is_finite() {
            // Curve that ends at start point isn't the case
            return false;
        }

        // Distance from point `(x0, y0)` to the line is `sqrt((c − a·x0 − b·y0)² / (a² + b²))`
        for i in (0..=(i - 2)).rev().step_by(2) {
            if (f64::powi(a * args[i] + b * args[i + 1], 2) * d) > (tolerance * tolerance) {
                return false;
            }
        }
        true
    }

    /// Returns the angle from the start of an arc to the end
    pub fn find_arc_angle(&self, rel_circle: &Circle) -> f64 {
        rel_circle.arc_angle(self)
    }

    /// Returns the distance of the start and end control points.
    pub fn control_point_distance_squared(&self, start: Point) -> (f64, f64) {
        let end = self.end_point();
        (
            control_point_distance_squared(self.start_control(), start, end),
            control_point_distance_squared(self.end_control(), start, end),
        )
    }

    /// Divides the curve into two halves drawn from some start point. Returns
    /// the left half and the right half with their starting points.
    pub fn subdivide(&self, start: Point) -> (Curve, Point, Curve) {
        self.subdivide_t(start, 0.5)
    }

    /// Returns two halves of the curve by a point that lies on the curve, up to some tolerance.
    pub fn subdivide_at(
        &self,
        start: Point,
        at: Point,
        tolerance: &ToleranceSquared,
    ) -> Option<(Curve, Curve)> {
        let result = self.subdivide_t(start, self.t_at(start, at, tolerance)?);
        Some((result.0, result.2))
    }

    /// Returns two divisions of the curve by the percentage along the curve, as a number
    /// between `0.0` and `1.0`.
    #[allow(clippy::similar_names)]
    pub fn subdivide_t(&self, start: Point, t: f64) -> (Curve, Point, Curve) {
        let p0 = start;
        let p1 = self.start_control();
        let p2 = self.end_control();
        let p3 = self.end_point();

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

    /// Returns the point `t` percent along a curve's chord, where `1.0` is `100%`
    #[must_use]
    #[deprecated]
    pub fn point_at(&self, t: f64) -> Point {
        self.point_at_from(Point::ZERO, t)
    }

    /// Returns the point `t` percent along a curve's chord from some
    /// start point, where `1.0` is `100%`
    pub fn point_at_from(&self, start: Point, t: f64) -> Point {
        let start_control = self.start_control();
        let end_control = self.end_control();
        let end_point = self.end_point();
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        mt3 * start + 3.0 * mt2 * t * start_control + 3.0 * mt * t2 * end_control + t3 * end_point
    }

    /// Returns the percent along the curve a point lies, to some tolerance.
    #[allow(clippy::similar_names)]
    pub fn t_at(&self, start: Point, at: Point, tolerance: &ToleranceSquared) -> Option<f64> {
        const MAX_ITER: usize = 8;
        let p0 = start;
        let p1 = self.start_control();
        let p2 = self.end_control();
        let p3 = self.end_point();

        let da = (p3 - p0) + (p1 - p2) * 3.0;
        let db = (p0 - p1 * 2.0 + p2) * 2.0;
        let dc = p1 - p0;

        let b_prime = |t: f64| -> Point {
            let t2 = t * t;
            (da * t2 + db * t + dc) * 3.0
        };
        let bd_prime = |t: f64| -> Point { (da * (2.0 * t) + db) * 3.0 };

        let tol_sq = **tolerance;

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

                let f1 = diff.dot(&bp);
                let f2 = bp.dot(&bp) + diff.dot(&bp2);

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

            let dist_sq = self.point_at_from(start, t).distance_squared(&at);
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
    #[must_use]
    pub fn reverse(&self, start: Point) -> Self {
        Curve::new(self.end_control(), self.start_control(), start)
    }

    /// Creates a subcurve between the percentage `t1` and `t2`, as numbers between `0.0` and `1.0`.
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
}

fn control_point_distance_squared(control: Point, start: Point, end: Point) -> f64 {
    let vector = end - start;
    let dot = vector.dot(&vector);
    if dot == 0.0 {
        return control.distance_squared(&start);
    }

    let t = ((control.0[0] - start.0[0]) * vector.0[0] + (control.0[1] - start.0[1]) * vector.0[1])
        / dot;
    let t = t.clamp(0.0, 1.0);
    let projection = Point([start.0[0] + t * vector.0[0], start.0[1] + t * vector.0[1]]);
    control.distance_squared(&projection)
}
