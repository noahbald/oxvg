use std::f64::consts::PI;

use crate::{geometry::Point, paths::segment::ToleranceSquared};

#[derive(Debug, Clone, Copy, PartialEq)]
/// An arc curve to some point.
pub struct Arc(pub [f64; 7]);

impl Arc {
    pub fn new(
        center: Point,
        radii: Point,
        start_angle: f64,
        sweep_angle: f64,
        x_rotation: f64,
    ) -> Self {
        Arc([
            center.x(),
            center.y(),
            radii.x(),
            radii.y(),
            start_angle,
            sweep_angle,
            x_rotation,
        ])
    }

    /// The center point of the ellipse
    pub const fn center(&self) -> Point {
        Point([self.0[0], self.0[1]])
    }

    /// The radii of the ellipse on each axis
    pub const fn radii(&self) -> Point {
        Point([self.0[2], self.0[3]])
    }

    /// The angle on the ellipse that the arc starts, in radians.
    /// Measured from the positive x-axis of the rotated ellipse
    /// coordinate system.
    pub const fn start_angle(&self) -> f64 {
        self.0[4]
    }

    /// The angular extent of the arc, in radians. Positive values
    /// indicate clockwise sweep, negative values counter-clockwise.
    pub const fn sweep_angle(&self) -> f64 {
        self.0[5]
    }

    /// The rotation angle of the ellipse's x-axis relative to the
    /// coordinate system's x-axis, in radians.
    pub const fn x_rotation(&self) -> f64 {
        self.0[6]
    }

    pub fn point_at_angle(&self, angle_radians: f64) -> Point {
        let radii = self.radii();
        let start = Point([
            radii.x() * angle_radians.cos(),
            radii.y() * angle_radians.sin(),
        ]);

        self.center() + start.rotate_radian(self.x_rotation())
    }

    pub fn start_point(&self) -> Point {
        self.point_at_angle(self.start_angle())
    }

    pub fn mid_point(&self) -> Point {
        self.point_at_angle(self.start_angle() + (self.sweep_angle() / 2.0))
    }

    pub fn end_point(&self) -> Point {
        self.point_at_angle(self.start_angle() + self.sweep_angle())
    }

    /// Converts `a` command parameters to [`Arc`].
    /// Returns `None` if the command can be replaced with [`Line`].
    ///
    /// Cursor is updated when `Some` is returned.
    pub fn from_arc_by(mut arc_by: [f64; 7], cursor: &mut Point) -> Option<Self> {
        arc_by[5] += cursor.0[0];
        arc_by[6] += cursor.0[1];
        Self::from_arc_to(arc_by, cursor)
    }

    /// Converts `A` command parameters to [`Arc`].
    /// Returns `None` if the command can be replaced with [`Line`].
    ///
    /// Cursor is updated when `Some` is returned.
    pub fn from_arc_to(arc_to: [f64; 7], from: &mut Point) -> Option<Self> {
        if is_arc_to_line(&arc_to, from) {
            return None;
        }

        // https://docs.rs/kurbo/0.13.0/src/kurbo/svg.rs.html#202-217
        let radii = Point([arc_to[0], arc_to[1]]);
        let x_rotation = arc_to[2].to_radians();
        let large_arc = arc_to[3];
        let sweep = arc_to[4];
        let to = Point([arc_to[5], arc_to[6]]);

        // https://docs.rs/kurbo/latest/src/kurbo/svg.rs.html#423-501
        let mut rx = radii.x().abs();
        let mut ry = radii.y().abs();

        let xr = x_rotation % (2.0 * PI);
        let (sin_phi, cos_phi) = xr.sin_cos();
        let hd_x = (from.x() - to.x()) * 0.5;
        let hd_y = (from.y() - to.y()) * 0.5;
        let hs_x = (from.x() + to.x()) * 0.5;
        let hs_y = (from.y() + to.y()) * 0.5;

        // F6.5.1
        let p = Point([
            cos_phi * hd_x + sin_phi * hd_y,
            -sin_phi * hd_x + cos_phi * hd_y,
        ]);

        // Sanitize the radii.
        // If rf > 1 it means the radii are too small for the arc to
        // possibly connect the end points. In this situation we scale
        // them up according to the formula provided by the SVG spec.

        // F6.6.2
        let rf = p.x() * p.x() / (rx * rx) + p.y() * p.y() / (ry * ry);
        if rf > 1.0 {
            let scale = rf.sqrt();
            rx *= scale;
            ry *= scale;
        }

        let rxry = rx * ry;
        let rxpy = rx * p.y();
        let rypx = ry * p.x();
        let sum_of_sq = rxpy * rxpy + rypx * rypx;

        debug_assert!(sum_of_sq != 0.0);

        // F6.5.2
        let sign_coe = if large_arc == sweep { -1.0 } else { 1.0 };
        let coe = sign_coe * ((rxry * rxry - sum_of_sq) / sum_of_sq).abs().sqrt();
        let transformed_cx = coe * rxpy / ry;
        let transformed_cy = -coe * rypx / rx;

        // F6.5.3
        let center = Point([
            cos_phi * transformed_cx - sin_phi * transformed_cy + hs_x,
            sin_phi * transformed_cx + cos_phi * transformed_cy + hs_y,
        ]);

        let start_v = Point([(p.x() - transformed_cx) / rx, (p.y() - transformed_cy) / ry]);
        let end_v = Point([
            (-p.x() - transformed_cx) / rx,
            (-p.y() - transformed_cy) / ry,
        ]);

        let start_angle = start_v.angle_radians();

        let mut sweep_angle = (end_v.angle_radians() - start_angle) % (2.0 * PI);

        if sweep != 0.0 && sweep_angle < 0.0 {
            sweep_angle += 2.0 * PI;
        } else if sweep == 0.0 && sweep_angle > 0.0 {
            sweep_angle -= 2.0 * PI;
        }

        *from = to;
        Some(Arc::new(
            center,
            Point([rx, ry]),
            start_angle,
            sweep_angle,
            x_rotation,
        ))
    }

    /// Converts the arc to `A` command parameters.
    pub fn to_arc_to(&self) -> [f64; 7] {
        let end = self.end_point();
        [
            self.radii().x(),
            self.radii().y(),
            self.x_rotation().to_degrees(),
            if self.sweep_angle().abs() > PI {
                1.0
            } else {
                0.0
            },
            if self.sweep_angle() > 0.0 { 1.0 } else { 0.0 },
            end.x(),
            end.y(),
        ]
    }

    pub fn subdivide(&self) -> (Arc, Arc) {
        self.subdivide_t(0.5)
    }

    pub fn is_straight(&self, error: f64) -> bool {
        self.radii().x() < error
            || self.radii().y() < error
            || Point::cross(self.start_point(), self.end_point(), self.mid_point()).abs() < error
    }

    pub fn t_at(&self, at: Point, tolerance: &ToleranceSquared) -> Option<f64> {
        let local = at - self.center();
        let unrotated = local.rotate(-self.x_rotation());
        let angle = unrotated
            .y()
            .atan2(unrotated.x() / self.radii().x() * self.radii().y());
        let mut delta = angle - self.start_angle();
        if self.sweep_angle() > 0.0 {
            delta = delta.rem_euclid(2.0 * PI);
        } else {
            delta = -((-delta).rem_euclid(2.0 * PI));
        }
        let t = (delta / self.sweep_angle()).clamp(0.0, 1.0);
        if self
            .point_at_angle(self.start_angle() + t * self.sweep_angle())
            .distance_squared(&at)
            <= **tolerance
        {
            Some(t)
        } else {
            None
        }
    }

    pub fn subdivide_at(&self, at: Point, tolerance: &ToleranceSquared) -> Option<(Arc, Arc)> {
        Some(self.subdivide_t(self.t_at(at, tolerance)?))
    }

    pub fn subdivide_t(&self, t: f64) -> (Arc, Arc) {
        let split_angle = self.start_angle() + t * self.sweep_angle();

        let mut left = *self;
        left.0[5] *= t;

        let mut right = *self;
        right.0[4] = split_angle;
        right.0[5] *= 1.0 - t;

        (left, right)
    }

    pub fn clamp_t(&self, t1: f64, t2: f64) -> Self {
        debug_assert!(t1 >= 0.0 && t1 <= 1.0);
        debug_assert!(t2 >= 0.0 && t2 <= 1.0);
        debug_assert!(t1 <= t2);
        let mut middle = *self;
        middle.0[4] = self.start_angle() + t1 * self.sweep_angle();
        middle.0[5] = (t2 - t1) * self.sweep_angle();
        middle
    }
}

fn is_arc_to_line(arc_to: &[f64; 7], cursor: &Point) -> bool {
    arc_to[0].abs() < 1e-5 || arc_to[1].abs() < 1e-5 || *cursor == Point([arc_to[5], arc_to[6]])
}
