#[cfg(feature = "deserialize")]
use serde::Deserialize;

use crate::{
    command::{self, Position},
    math,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Point(pub [f64; 2]);

#[derive(Debug, Clone)]
pub struct Curve(pub [f64; 6]);

#[derive(Debug, Clone)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

#[cfg_attr(feature = "deserialize", derive(Deserialize))]
#[derive(Clone)]
pub struct MakeArcs {
    pub threshold: f64,
    pub tolerance: f64,
}

impl Curve {
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

    pub fn is_arc(&self, circle: &Circle, make_arcs: &MakeArcs, error: f64) -> bool {
        let tolerance =
            (make_arcs.threshold * error).min((make_arcs.tolerance * circle.radius) / 100.0);
        [0.0, 0.25, 0.5, 0.75, 1.0].into_iter().all(|t| {
            (Point::cubic_bezier(self, t).distance(&circle.center) - circle.radius).abs()
                <= tolerance
        })
    }

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

    pub fn is_straight(&self, error: f64) -> bool {
        Self::is_data_straight(&self.0, error)
    }

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

    fn distance(&self, other: &Point) -> f64 {
        math::hypot(self.0[0] - other.0[0], self.0[1] - other.0[1])
    }

    pub fn reflect(&self, base: Self) -> Self {
        Self([2.0 * base.0[0] - self.0[0], 2.0 * base.0[1] - self.0[1]])
    }
}

impl Circle {
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

    pub fn arc_angle(&self, curve: &Curve) -> f64 {
        let x1 = -self.center.0[0];
        let y1 = -self.center.0[1];
        let x2 = curve.0[4] - self.center.0[0];
        let y2 = curve.0[4] - self.center.0[1];
        f64::acos((x1 * x2 + y1 * y2) / f64::sqrt((x1 * x1 + y1 * y1) * (x2 * x2 + y2 * y2)))
    }
}
