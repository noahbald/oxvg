use std::f64::consts::PI;

use crate::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
/// An arc curve to some point.
pub struct Arc(pub [f64; 7]);

impl Arc {
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
    /// coordinate system's x-axis, in degrees.
    pub const fn x_rotation(&self) -> f64 {
        self.0[6]
    }

    pub fn point_at_angle(&self, angle_radians: f64) -> Point {
        let end_angle = self.start_angle() + angle_radians;
        let radii = self.radii();
        let start = Point([radii.x() * end_angle.cos(), radii.y() * end_angle.sin()]);

        self.center() + start.rotate(self.x_rotation())
    }

    pub fn mid_point(&self) -> Point {
        self.point_at_angle(self.start_angle().midpoint(self.sweep_angle()))
    }

    pub fn end_point(&self) -> Point {
        self.point_at_angle(self.sweep_angle())
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
    pub fn from_arc_to(arc_to: [f64; 7], cursor: &mut Point) -> Option<Self> {
        // https://docs.rs/kurbo/latest/src/kurbo/svg.rs.html#423-501
        if is_arc_to_line(&arc_to, cursor) {
            return None;
        }
        let mut r = Point([arc_to[0], arc_to[1]]);
        let angle = arc_to[2];
        let large_arc_flag = arc_to[3];
        let sweep_flag = arc_to[4];
        let end = Point([arc_to[5], arc_to[6]]);

        let hd = (*cursor - end) / 2.0;
        let hs = (*cursor + end) / 2.0;

        let p = hd.rotate(angle);

        let rf = (p * p) / (r * r);
        let rf = rf.x() + rf.y();
        if rf > 1.0 {
            r = r * rf.sqrt();
        }

        let rxry = r.x() * r.y();
        let rxpy = r.x() * p.y();
        let rypx = r.y() * p.x();
        let sum_of_sq = rxpy * rxpy + rypx * rypx;

        debug_assert!(sum_of_sq != 0.0);

        let sign_coe = if large_arc_flag == sweep_flag {
            -1.0
        } else {
            1.0
        };
        let coe = sign_coe * ((rxry * rxry - sum_of_sq) / sum_of_sq).abs().sqrt();
        let transformed = Point([coe * rxpy / r.y(), -coe * rypx / r.x()]);

        // F6.5.3
        let center = transformed.rotate(angle) + hs;

        let start_v = (p - transformed) / r;
        let end_v = (p.minus() - transformed) / r;

        let start_angle = start_v.angle_radians();
        let mut sweep_angle = (end_v.angle_radians() - start_angle) % (2.0 * PI);

        if sweep_flag != 0.0 && sweep_angle < 0.0 {
            sweep_angle += 2.0 * PI;
        } else if sweep_flag == 0.0 && sweep_angle > 0.0 {
            sweep_angle -= 2.0 * PI;
        }

        *cursor = end;
        Some(Self([
            center.x(),
            center.y(),
            r.x(),
            r.y(),
            start_angle,
            sweep_angle,
            arc_to[2],
        ]))
    }

    pub fn subdivide(&self, start: Point) -> ((Point, Arc), (Point, Arc)) {
        let mut left = *self;
        left.0[5] /= 2.0;

        let mut right = left;
        right.0[4] += right.0[5];
        ((start, left), (self.mid_point(), right))
    }

    pub fn is_straight(&self, error: f64) -> bool {
        self.radii().x() < error || self.radii().y() < error
    }
}

fn is_arc_to_line(arc_to: &[f64; 7], cursor: &Point) -> bool {
    arc_to[0].abs() < 1e-5 || arc_to[1].abs() < 1e-5 || *cursor == Point([arc_to[5], arc_to[6]])
}
