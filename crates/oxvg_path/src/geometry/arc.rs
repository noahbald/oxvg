//! Types for representing elliptical arcs.
use std::{cell::Cell, f64::consts::PI};

use crate::{
    geometry::{ellipses::Ellipses, Curve, Point, Quadrant},
    math::{self, radius_factor},
    optimize::Tolerance,
    paths::segment::{TolerancePrecision, ToleranceSquared},
};

#[derive(Clone, PartialEq)]
/// An arc curve to some point.
pub struct Arc {
    center: Point,
    radii: Point,
    start_angle: f64,
    sweep_angle: f64,
    x_rotation: f64,
    // TODO: private internals; invalid memo on mutation; debug_assert memo
    end_point_memo: Cell<Option<Point>>,
}

impl Arc {
    /// Creates an arc
    pub const fn new(
        center: Point,
        radii: Point,
        start_angle: f64,
        sweep_angle: f64,
        x_rotation: f64,
    ) -> Self {
        Arc {
            center,
            radii,
            start_angle,
            sweep_angle,
            x_rotation,
            end_point_memo: Cell::new(None),
        }
    }

    /// The center point of the ellipse
    pub const fn center(&self) -> Point {
        self.center
    }

    /// The radii of the ellipse on each axis
    pub const fn radii(&self) -> Point {
        self.radii
    }

    /// The angle on the ellipse that the arc starts, in radians.
    /// Measured from the positive x-axis of the rotated ellipse
    /// coordinate system.
    pub const fn start_angle(&self) -> f64 {
        self.start_angle
    }

    /// The angular extent of the arc, in radians. Positive values
    /// indicate clockwise sweep, negative values counter-clockwise.
    pub const fn sweep_angle(&self) -> f64 {
        self.sweep_angle
    }

    /// The rotation angle of the ellipse's x-axis relative to the
    /// coordinate system's x-axis, in radians.
    pub const fn x_rotation(&self) -> f64 {
        self.x_rotation
    }

    pub fn set_center(&mut self, center: Point) {
        self.end_point_memo.set(None);
        self.center = center;
    }

    pub fn set_radii(&mut self, radii: Point) {
        self.end_point_memo.set(None);
        self.radii = radii;
    }

    pub fn set_start_angle(&mut self, start_angle: f64) {
        self.end_point_memo.set(None);
        self.start_angle = start_angle;
    }

    pub fn set_sweep_angle(&mut self, sweep_angle: f64) {
        self.end_point_memo.set(None);
        self.sweep_angle = sweep_angle;
    }

    pub fn set_x_rotation(&mut self, x_rotation: f64) {
        self.end_point_memo.set(None);
        self.x_rotation = x_rotation;
    }

    /// Returns the point on the ellipses of the arc's start point.
    pub fn start_point(&self) -> Point {
        self.point_at_angle(self.start_angle())
    }

    /// Returns the point on the ellipses halfway between the arc's start and end point.
    pub fn mid_point(&self) -> Point {
        self.point_at_angle(self.start_angle() + (self.sweep_angle() / 2.0))
    }

    /// Returns the point on the ellipses of the arc's end point.
    pub fn end_point(&self) -> Point {
        if let Some(end_point) = self.end_point_memo.get() {
            end_point
        } else {
            let end_point = self.point_at_angle(self.start_angle + self.sweep_angle);
            self.end_point_memo.set(Some(end_point));
            end_point
        }
    }

    pub fn set_end_point_memo(&mut self, end_point: Point) {
        self.end_point_memo.set(None);
        debug_assert!(
            self.end_point().distance_squared(end_point) < 5e-2,
            "Memoised end-point ({end_point:?}) out of range of computed end-point({:?}) by {}",
            self.end_point(),
            self.end_point().distance(end_point)
        );
        self.end_point_memo.set(Some(end_point));
    }

    #[must_use]
    pub fn with_end_point_memo(mut self, end_point: Point) -> Self {
        self.set_end_point_memo(end_point);
        self
    }

    /// Returns the ellipsess the arc sweeps across.
    pub const fn ellipses(&self) -> Ellipses {
        Ellipses::new(self.center, self.radii, self.x_rotation)
    }

    /// Returns the point on the ellipses at the given angle.
    pub fn point_at_angle(&self, angle_radians: f64) -> Point {
        self.ellipses().point_at_angle(angle_radians)
    }

    /// Returns the approximate perimeter of the arc, using Ramanujan's algorithm.
    #[allow(clippy::cast_precision_loss)]
    pub fn len(&self, iterations: usize) -> f64 {
        if self.radii().x == 0.0 || self.radii().y == 0.0 {
            return self.start_point().distance(self.end_point());
        }
        if self.sweep_angle() == 0.0 {
            return 0.0;
        }

        let iterations = if iterations.is_multiple_of(2) {
            iterations.max(2)
        } else {
            iterations + 1
        };

        let mut sum = 0.0;
        let h = self.sweep_angle() / (iterations as f64);
        for i in 0..iterations {
            let theta = self.start_angle() + (i as f64 * h);
            let integrand = ((self.radii().x.powi(2) * theta.sin().powi(2))
                + (self.radii().y.powi(2) * theta.cos().powi(2)))
            .sqrt();
            if i % 2 == 0 {
                sum += 2.0 * integrand;
            } else {
                sum += 4.0 * integrand;
            }
        }

        ((sum * h) / 3.0).abs()
    }

    /// Converts `a` command parameters to [`Arc`].
    /// Returns `None` if the command can be replaced with [`Line`].
    ///
    /// Cursor is updated when `Some` is returned.
    pub fn from_arc_by(mut arc_by: [f64; 7], cursor: &mut Point) -> Option<Self> {
        arc_by[5] += cursor.x;
        arc_by[6] += cursor.y;
        Self::from_arc_to(arc_by, cursor)
    }

    /// Converts `A` command parameters to [`Arc`].
    /// Returns `None` if the command can be replaced with [`Line`].
    ///
    /// Cursor is updated when `Some` is returned.
    #[allow(clippy::similar_names)]
    pub fn from_arc_to(arc_to: [f64; 7], from: &mut Point) -> Option<Self> {
        if is_arc_to_line(&arc_to, from) {
            return None;
        }

        // https://docs.rs/kurbo/0.13.0/src/kurbo/svg.rs.html#202-217
        let radii = Point::new(arc_to[0], arc_to[1]);
        let x_rotation = arc_to[2].to_radians();
        let large_arc = arc_to[3];
        let sweep = arc_to[4];
        let to = Point::new(arc_to[5], arc_to[6]);

        // https://docs.rs/kurbo/latest/src/kurbo/svg.rs.html#423-501
        let mut rx = radii.x.abs();
        let mut ry = radii.y.abs();

        let xr = x_rotation % (2.0 * PI);
        let (sin_phi, cos_phi) = xr.sin_cos();
        let hd_x = (from.x - to.x) * 0.5;
        let hd_y = (from.y - to.y) * 0.5;
        let hs_x = (from.x + to.x) * 0.5;
        let hs_y = (from.y + to.y) * 0.5;

        // F6.5.1
        let p = Point::new(
            cos_phi * hd_x + sin_phi * hd_y,
            -sin_phi * hd_x + cos_phi * hd_y,
        );

        // Sanitize the radii.
        // If rf > 1 it means the radii are too small for the arc to
        // possibly connect the end points. In this situation we scale
        // them up according to the formula provided by the SVG spec.

        // F6.6.2
        let rf = p.x * p.x / (rx * rx) + p.y * p.y / (ry * ry);
        if rf > 1.0 {
            let scale = rf.sqrt();
            rx *= scale;
            ry *= scale;
        }

        let rxry = rx * ry;
        let rxpy = rx * p.y;
        let rypx = ry * p.x;
        let sum_of_sq = rxpy * rxpy + rypx * rypx;

        debug_assert!(sum_of_sq != 0.0);

        // F6.5.2
        let sign_coe = if large_arc == sweep { -1.0 } else { 1.0 };
        let coe = sign_coe * ((rxry * rxry - sum_of_sq) / sum_of_sq).abs().sqrt();
        let transformed_cx = coe * rxpy / ry;
        let transformed_cy = -coe * rypx / rx;

        // F6.5.3
        let center = Point::new(
            cos_phi * transformed_cx - sin_phi * transformed_cy + hs_x,
            sin_phi * transformed_cx + cos_phi * transformed_cy + hs_y,
        );

        let start_v = Point::new((p.x - transformed_cx) / rx, (p.y - transformed_cy) / ry);
        let end_v = Point::new((-p.x - transformed_cx) / rx, (-p.y - transformed_cy) / ry);

        let start_angle = start_v.angle_radians();

        let mut sweep_angle = (end_v.angle_radians() - start_angle) % (2.0 * PI);

        if sweep != 0.0 && sweep_angle < 0.0 {
            sweep_angle += 2.0 * PI;
        } else if sweep == 0.0 && sweep_angle > 0.0 {
            sweep_angle -= 2.0 * PI;
        }

        *from = to;
        Some(
            Arc::new(
                center,
                Point::new(rx, ry),
                start_angle,
                sweep_angle,
                x_rotation,
            )
            .with_end_point_memo(to),
        )
    }

    /// Returns whether the arc approximately fits on a circle.
    pub fn is_circle(&self, tolerance: &Tolerance) -> bool {
        self.ellipses().is_circle(tolerance)
    }

    pub(crate) fn is_connected(
        &self,
        other: &Self,
        tolerance: &Tolerance,
        tolerance_squared: &ToleranceSquared,
    ) -> bool {
        let min_sweep = self.sweep_angle().abs().min(other.sweep_angle().abs());
        if self.radii().x == self.radii().y
            && other.radii().x == other.radii().y
            && self.sweep_angle().signum() == other.sweep_angle().signum()
        {
            let max_radius = self.radii().x.max(other.radii().x);
            let straight_threshold = 0.1 * max_radius;
            if min_sweep < straight_threshold {
                // Relatively small, relatively round arcs can be regarded as continuous
                return true;
            }
        }

        let scale = (min_sweep.powi(-1)).max(40.0 * self.radii().len());
        let tolerance_scaled = **tolerance_squared * scale;
        self.center().distance_squared(other.center()) < tolerance_scaled
            && self.radii().distance_squared(other.radii()) < tolerance_scaled
            && (
                // TODO: Check x_rotation affects start_angle
                (self.radii().x - self.radii().y).abs() < **tolerance_squared
                    || (self.x_rotation() - other.x_rotation()).abs() < tolerance.angular
            )
    }

    /// Returns an approximate equivalent of the arc as a bezier curve, if possible.
    pub fn to_curve_to(&self) -> Option<Curve> {
        if self.sweep_angle().abs() < std::f64::consts::FRAC_PI_2 + 1e-6 {
            return None;
        }

        let sweep = self.sweep_angle();
        let k = (4.0 / 3.0) * (sweep / 4.0).tan();

        let (sin_start, cos_start) = self.start_angle().sin_cos();
        let (sin_end, cos_end) = (self.start_angle() + sweep).sin_cos();

        let rx = self.radii().x;
        let ry = self.radii().y;
        let (sin_rot, cos_rot) = self.x_rotation().sin_cos();

        let d_start = Point::new(
            k * (-rx * sin_start * cos_rot - ry * cos_start * sin_rot),
            k * (-rx * sin_start * sin_rot + ry * cos_start * cos_rot),
        );
        let d_end = Point::new(
            k * (-rx * sin_end * cos_rot - ry * cos_end * sin_rot),
            k * (-rx * sin_end * sin_rot + ry * cos_end * cos_rot),
        );

        let start = self.start_point();
        let end = self.end_point();

        Some(Curve::new(start + d_start, end - d_end, end))
    }

    /// Converts the arc to `A` command parameters.
    pub fn to_arc_to(
        &self,
        tolerance: &Tolerance,
        tolerance_squared: &ToleranceSquared,
        precision: &TolerancePrecision,
    ) -> [f64; 7] {
        let end = self.end_point();
        let mut radii = self.radii();
        let lambda = self.radius_factor();
        let limit = self.radii() * lambda.sqrt();
        if (self.radius_factor() > 1.0)
            || ((self.radii() * lambda.sqrt()).distance_squared(self.radii()) < **tolerance_squared)
        {
            let denom = math::euclid_gcd_lossy(radii.x, radii.y, tolerance, precision);
            let mut simple = radii / denom;
            while (simple - limit).quadrant() == Quadrant::A {
                simple = simple / 2.0;
            }
            radii = simple;
        }
        [
            radii.x,
            radii.y,
            self.x_rotation().to_degrees(),
            if self.sweep_angle().abs() > PI {
                1.0
            } else {
                0.0
            },
            if self.sweep_angle() > 0.0 { 1.0 } else { 0.0 },
            end.x,
            end.y,
        ]
    }

    /// For a curve that fits an arc with some tolerance, returns the equivalent arc.
    pub fn fit_curve(
        curve: &Curve,
        start: Point,
        tolerance: &Tolerance,
        tolerance_squared: &ToleranceSquared,
    ) -> Option<Self> {
        let ellipses = Ellipses::fit_curve(curve, start, tolerance)?;
        let tolerance = ellipses.ellipse_tolerance(tolerance_squared);
        let start_angle = ellipses.angle_at_point(start, &tolerance)?;
        let end_angle = ellipses.angle_at_point(curve.end_point, &tolerance)?;
        let mut sweep_angle = end_angle - start_angle;

        if sweep_angle > PI {
            sweep_angle -= 2.0 * PI;
        } else if sweep_angle < -PI {
            sweep_angle += 2.0 * PI;
        }

        Some(
            ellipses
                .arc(start_angle, sweep_angle)
                .with_end_point_memo(curve.end_point),
        )
    }

    /// Returns two halves of the arc, with the left being the first half and the right being the second half.
    pub fn subdivide(&self) -> (Arc, Arc) {
        self.subdivide_t(0.5)
    }

    /// Returns whether the arc is equivalent to a straight line within some tolerance.
    pub fn is_straight(&self, tolerance: &Tolerance) -> bool {
        self.radii().x < tolerance.positional
            || self.radii().y < tolerance.positional
            || self
                .start_point()
                .cross(self.end_point(), self.mid_point())
                .abs()
                / self.start_point().distance(self.end_point()).max(1.0)
                < tolerance.positional
    }

    /// Returns the percentage as a number between `0.0` and `1.0` that the given point
    /// is between the arc's start and end point, when that point is on the arc within
    /// some tolerance.
    pub fn t_at(&self, at: Point, tolerance: &ToleranceSquared) -> Option<f64> {
        let ellipses = self.ellipses();
        let tolerance = ellipses.ellipse_tolerance(tolerance);
        let angle = self.ellipses().angle_at_point(at, &tolerance)?;
        let mut delta = angle - self.start_angle();
        if self.sweep_angle() > 0.0 {
            delta = delta.rem_euclid(2.0 * PI);
        } else {
            delta = -((-delta).rem_euclid(2.0 * PI));
        }
        let t = (delta / self.sweep_angle()).clamp(0.0, 1.0);
        if self
            .point_at_angle(self.start_angle() + t * self.sweep_angle())
            .distance_squared(at)
            <= *tolerance
        {
            Some(t)
        } else {
            None
        }
    }

    /// Returns two divisions of the arc by some point between the arc's start and end, with the left
    /// being the first division and the right being the second division.
    pub fn subdivide_at(&self, at: Point, tolerance: &ToleranceSquared) -> Option<(Arc, Arc)> {
        Some(self.subdivide_t(self.t_at(at, tolerance)?))
    }

    /// Returns two divisions of the arc by some percentage between the arc's start and end as a number
    /// between `0.0` and `1.0`, with the left being the first division and the right being
    /// the second division.
    pub fn subdivide_t(&self, t: f64) -> (Arc, Arc) {
        let split_angle = self.start_angle + t * self.sweep_angle;

        let mut left = self.clone();
        left.set_sweep_angle(t * self.sweep_angle);

        let mut right = self.clone();
        right.set_start_angle(split_angle);
        right.set_sweep_angle(self.sweep_angle * (1.0 - t));

        (left, right)
    }

    /// Returns the sub-arc of this arc between the two percentages.
    #[must_use]
    pub fn clamp_t(&self, t1: f64, t2: f64) -> Self {
        debug_assert!((0.0..=1.0).contains(&t1));
        debug_assert!((0.0..=1.0).contains(&t2));
        debug_assert!(t1 <= t2);
        let mut middle = self.clone();
        middle.set_start_angle(self.start_angle() + t1 * self.sweep_angle());
        middle.set_sweep_angle((t2 - t1) * self.sweep_angle());
        middle
    }

    /// Returns an arc spanning from the end to start of this arc
    #[must_use]
    pub fn reverse(&self) -> Self {
        Arc::new(
            self.center(),
            self.radii(),
            self.start_angle() + self.sweep_angle(),
            -self.sweep_angle(),
            self.x_rotation(),
        )
    }

    /// Returns the radius factor as defined by the SVG spec (F6.6.2)
    pub fn radius_factor(&self) -> f64 {
        radius_factor(
            self.radii().x,
            self.radii().y,
            self.x_rotation(),
            self.start_point(),
            self.end_point(),
        )
    }
}

impl std::fmt::Debug for Arc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arc")
            .field("center", &self.center())
            .field("radii", &self.radii())
            .field("start_angle", &self.start_angle())
            .field("sweep_angle", &self.sweep_angle())
            .field("x_rotation", &self.x_rotation())
            .field("start_point (computed)", &self.start_point())
            .field("end_point (computed)", &self.end_point())
            .finish()
    }
}

fn is_arc_to_line(arc_to: &[f64; 7], cursor: &Point) -> bool {
    arc_to[0].abs() < 1e-5 || arc_to[1].abs() < 1e-5 || *cursor == Point::new(arc_to[5], arc_to[6])
}
