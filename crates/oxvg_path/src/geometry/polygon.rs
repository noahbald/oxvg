use crate::geometry::{Curve, Point};

/// A polygon is a list of points
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub points: Vec<Point>,
    pub closed: bool,
}

impl Polygon {
    pub fn from_curve(points: &mut Vec<Point>, start_point: Point, curve: Curve, tolerance: f64) {
        let (start_control_distance, end_control_distance) =
            curve.control_point_distance(start_point);

        if start_control_distance <= tolerance && end_control_distance <= tolerance {
            points.push(curve.end_point());
        } else {
            let ((start, left), (middle, right)) = curve.subdivide(start_point);
            Self::from_curve(points, start, left, tolerance);
            Self::from_curve(points, middle, right, tolerance);
        }
    }
}
