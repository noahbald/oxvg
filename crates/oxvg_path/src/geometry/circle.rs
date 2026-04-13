use crate::geometry::{Curve, ErrorOptions, Line, Point};

#[derive(Debug, Clone)]
/// A circle shape
pub struct Circle {
    /// The centre point of the circle.
    pub center: Point,
    /// The length from the centre of the circle to the edge.
    pub radius: f64,
}

impl Circle {
    /// From a curve, which is potentially an arc, find the corresponding circle
    pub fn find(curve: &Curve, make_arcs: &ErrorOptions, error: f64) -> Option<Self> {
        let mid_point = Point::cubic_bezier(curve, 0.5);
        let m1 = mid_point / 2.0;
        let m2 = Line([m1, curve.end_point()]).midpoint();
        let l1 = Line([m1, m1 + Point([m1.y(), -m1.x()])]);
        let l2 = Line([
            m2,
            m2 + Point([m2.y() - mid_point.y(), -(m2.x() - mid_point.x())]),
        ]);

        let center = l1.intersection(&l2)?;
        let radius = center.distance(&Point([0.0; 2]));
        let tolerance = (make_arcs.threshold * error).min((make_arcs.tolerance * radius) / 100.0);

        if radius < 1e15
            && [0.25, 0.75].into_iter().all(|t| {
                (Point::cubic_bezier(curve, t).distance(&center) - radius).abs() <= tolerance
            })
        {
            Some(Circle { center, radius })
        } else {
            None
        }
    }

    /// Returns the angle of a curve fitting the circle
    pub fn arc_angle(&self, curve: &Curve) -> f64 {
        let v1 = self.center * -1.0;
        let v2 = curve.end_point() - self.center;
        let dot = v1.dot(&v2);
        let len = v1.len() * v2.len();
        f64::acos(dot / len)
    }
}
