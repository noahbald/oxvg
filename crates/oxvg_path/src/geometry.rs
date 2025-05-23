//! Type of shaped used for processing path data.
use crate::{
    command::{self, Position},
    math,
};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
/// A point is an `[x, y]` coordinate
pub struct Point(pub [f64; 2]);

#[derive(Debug, Clone)]
/// A bezier curve.
///
/// For an absolute, command
/// `"C x1 y1 x2 y2 x y"`
///
/// Or, for a relative command
/// `"c dx1 dy1 dx2 dy2 dx, dy"`
///
/// The point `[x y]` specifies where the curve should end.
///
/// The points `[x1 y1]` and `[x2 y2]` are the control points. The former being for controlling the
/// start of the curve and the latter controlling the end.
pub struct Curve(pub [f64; 6]);

#[derive(Debug, Clone)]
/// A circle shape
pub struct Circle {
    /// The centre point of the circle.
    pub center: Point,
    /// The length from the centre of the circle to the edge.
    pub radius: f64,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
/// When running calculations against arcs, the level of error tolerated
pub struct MakeArcs {
    /// When calculating tolerance, controls the bound compared to error
    pub threshold: f64,
    /// When calculating tolerance, controls the bound compared to the radius
    pub tolerance: f64,
}

impl Default for MakeArcs {
    fn default() -> Self {
        Self {
            threshold: 2.5,
            tolerance: 0.5,
        }
    }
}

impl Curve {
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

    /// Returns whether a curve is convex
    ///
    /// A curve is convex when the middle of the curve's line is below the curve's midpoint
    pub fn is_convex(&self) -> bool {
        let data = self.0;
        let Some(center) = Point::intersection([
            0.0, 0.0, data[2], data[3], data[0], data[1], data[4], data[5],
        ]) else {
            return false;
        };
        let center = center.0;
        (data[2] < center[0]) == (center[0] < 0.0)
            && (data[3] < center[1]) == (center[1] < 0.0)
            && (data[4] < center[0]) == (center[0] < data[0])
            && (data[5] < center[1]) == (center[1] < data[1])
    }

    /// Returns whether a curve is an arc of a circle
    pub fn is_arc(&self, circle: &Circle, make_arcs: &MakeArcs, error: f64) -> bool {
        let tolerance = f64::min(
            make_arcs.threshold * error,
            (make_arcs.tolerance * circle.radius) / 100.0,
        );
        [0.0, 0.25, 0.5, 0.75, 1.0].into_iter().all(|t| {
            (Point::cubic_bezier(self, t).distance(&circle.center) - circle.radius).abs()
                <= tolerance
        })
    }

    /// Returns whether a curve from a previous command is an arc of a circle
    pub fn is_arc_prev(&self, circle: &Circle, make_arcs: &MakeArcs, error: f64) -> bool {
        let center = circle.center.0;
        self.is_arc(
            &Circle {
                center: Point([center[0] + self.0[4], center[1] + self.0[5]]),
                radius: circle.radius,
            },
            make_arcs,
            error,
        )
    }

    /// Returns whether the arc fits on a straight line
    pub fn is_straight(&self, error: f64) -> bool {
        Self::is_data_straight(&self.0, error)
    }

    /// Returns whether the arc fits on a straight line
    pub fn is_data_straight(args: &[f64], error: f64) -> bool {
        // Get line equation a·x + b·y + c = 0 coefficients a, b (c = 0) by start and end points.
        let i = args.len() - 2;
        let a = -args[i + 1]; // y1 − y2 (y1 = 0)
        let b = args[i]; // x2 − x1 (x1 = 0)
        let d = 1.0 / (a * a + b * b); // same part for all points

        if i <= 1 || !d.is_finite() {
            // curve that ends at start point isn't the case
            return false;
        }

        // Distance from point (x0, y0) to the line is sqrt((c − a·x0 − b·y0)² / (a² + b²))
        for i in (0..=(i - 2)).rev().step_by(2) {
            if f64::sqrt(f64::powi(a * args[i] + b * args[i + 1], 2) * d) > error {
                return false;
            }
        }
        true
    }

    /// Returns the angle from the start of an arc to the end
    pub fn find_arc_angle(&self, rel_circle: &Circle) -> f64 {
        rel_circle.arc_angle(self)
    }
}

impl Point {
    /// Gets the point at which two lines cross
    /// [
    ///     // line 1
    ///     x1, y1, x2, y2,
    ///     // line 2
    ///     x1, y2, x2, y2,
    /// ]
    fn intersection(coords: [f64; 8]) -> Option<Self> {
        let a1 = coords[1] - coords[3]; // y1 - y2
        let b1 = coords[2] - coords[0]; // x2 - x1
        let c1 = coords[0] * coords[3] - coords[2] * coords[1]; // x1 * y2 - x2 * y1
                                                                // Next line equation parameters
        let a2 = coords[5] - coords[7]; // y1 - y2
        let b2 = coords[6] - coords[4]; // x2 - x1
        let c2 = coords[4] * coords[7] - coords[5] * coords[6]; // x1 * y2 - x2 * y1
        let denom = a1 * b2 - a2 * b1;

        if denom == 0.0 {
            return None;
        }
        let cross = [(b1 * c2 - b2 * c1) / denom, (a1 * c2 - a2 * c1) / -denom];
        if cross[0].is_nan() || cross[1].is_nan() || !cross[0].is_finite() || !cross[1].is_finite()
        {
            return None;
        }
        Some(Self(cross))
    }

    /// Returns the point `t` percent along a curve's chord
    fn cubic_bezier(curve: &Curve, t: f64) -> Self {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;

        Self([
            3.0 * mt2 * t * curve.0[0] + 3.0 * mt * t2 * curve.0[2] + t3 * curve.0[4],
            3.0 * mt2 * t * curve.0[1] + 3.0 * mt * t2 * curve.0[3] + t3 * curve.0[5],
        ])
    }

    /// Returns the distance between two points
    fn distance(&self, other: &Point) -> f64 {
        math::hypot(self.0[0] - other.0[0], self.0[1] - other.0[1])
    }

    /// Creates a point diagonally across from another point
    pub fn reflect(&self, base: Self) -> Self {
        Self([2.0 * base.0[0] - self.0[0], 2.0 * base.0[1] - self.0[1]])
    }

    /// The subtraction of two points
    pub fn sub(&self, Self(v2): Self) -> Self {
        Self([self.0[0] - v2[0], self.0[1] - v2[1]])
    }

    /// The dot product of two points
    pub fn dot(&self, Self(v2): &Self) -> f64 {
        self.0[0] * v2[0] + self.0[1] * v2[1]
    }

    /// The cross product of two points
    pub fn cross(Self(o): Self, Self(a): Self, Self(b): &Self) -> f64 {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    }

    /// The inverse of a point, by multiplying by `-1`
    pub fn minus(&self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }

    /// The orthogonal of a point
    pub fn orth(&self, from: &Self) -> Self {
        let o = Self([-self.0[1], self.0[0]]);
        if o.dot(&from.minus()) < 0.0 {
            o.minus()
        } else {
            o
        }
    }

    /// As part of the GJK algorithm, takes the current simplex (a polygon) and search direction
    /// in order to find the direction and subset of the simplex to try next.
    pub fn process_simplex(simplex: &mut Vec<Self>, Self(direction): &mut Self) -> bool {
        // We only need to handle to 1-simplex and 2-simplex
        if simplex.len() == 2 {
            let a = simplex[1];
            let b = simplex[0];
            let ao = a.minus();
            let ab = b.sub(a);
            // ao is in the same direction as ab
            if ao.dot(&ab) > 0.0 {
                // get the vector perpendicular to ab facing o
                *direction = ab.orth(&a).0;
            } else {
                *direction = ao.0;
                // only a remains in the simplex
                simplex.remove(0);
            }
        } else {
            // 2-simplex
            let a = simplex[2];
            let b = simplex[1];
            let c = simplex[0];
            let ab = b.sub(a);
            let ac = c.sub(a);
            let ao = a.minus();
            let acb = ab.orth(&ac); // The vector perpendicular to ab facing away from c
            let abc = ac.orth(&ab); // The vector perpendicular to ac facing away from b

            if acb.dot(&ao) > 0.0 {
                if ab.dot(&ao) > 0.0 {
                    // region 4
                    *direction = acb.0;
                    simplex.remove(0);
                } else {
                    // region 5
                    *direction = ao.0;
                    simplex.drain(0..=1);
                }
            } else if abc.dot(&ao) > 0.0 {
                if ac.dot(&ao) > 0.0 {
                    // region 6
                    *direction = abc.0;
                    simplex.remove(1);
                } else {
                    // region 5 (again)
                    *direction = ao.0;
                    simplex.drain(0..=1);
                }
            } else {
                return true;
            }
        }
        false
    }
}

impl Circle {
    /// From a curve, which is potentially an arc, find the correspoding circle
    pub fn find(curve: &Curve, make_arcs: &MakeArcs, error: f64) -> Option<Self> {
        let mid_point = Point::cubic_bezier(curve, 0.5).0;
        let m1 = [mid_point[0] / 2.0, mid_point[1] / 2.0];
        let m2 = [
            (mid_point[0] + curve.0[4]) / 2.0,
            (mid_point[1] + curve.0[5]) / 2.0,
        ];
        let center = Point::intersection([
            m1[0],
            m1[1],
            m1[0] + m1[1],
            m1[1] - m1[0],
            m2[0],
            m2[1],
            m2[0] + (m2[1] - mid_point[1]),
            m2[1] - (m2[0] - mid_point[0]),
        ])?;
        let radius = center.distance(&Point([0.0; 2]));
        let tolerance = (make_arcs.threshold * error).min((make_arcs.tolerance * radius) / 100.0);

        if radius < 1e15
            && [0.25, 0.75].into_iter().all(|t| {
                (Point::cubic_bezier(curve, t).distance(&center) - radius).abs() <= tolerance
            })
        {
            return Some(Circle { center, radius });
        }
        None
    }

    /// Returns the angle of a curve fitting the circle
    pub fn arc_angle(&self, curve: &Curve) -> f64 {
        let x1 = -self.center.0[0];
        let y1 = -self.center.0[1];
        let x2 = curve.0[4] - self.center.0[0];
        let y2 = curve.0[5] - self.center.0[1];
        f64::acos((x1 * x2 + y1 * y2) / f64::sqrt((x1 * x1 + y1 * y1) * (x2 * x2 + y2 * y2)))
    }
}
