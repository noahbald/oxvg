use crate::{
    geometry::{Arc, Curve, Point},
    paths::segment::ToleranceSquared,
};

/// A polygon is a list of points
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub points: Vec<Point>,
    pub closed: bool,
}

impl Polygon {
    pub fn from_curve(
        points: &mut Vec<Point>,
        start_point: Point,
        curve: &Curve,
        tolerance_squared: &ToleranceSquared,
    ) {
        let (start_control_distance, end_control_distance) =
            curve.control_point_distance_squared(start_point);

        if start_control_distance <= **tolerance_squared
            && end_control_distance <= **tolerance_squared
        {
            points.push(curve.end_point());
        } else {
            let ((start, left), (middle, right)) = curve.subdivide(start_point);
            Self::from_curve(points, start, &left, tolerance_squared);
            Self::from_curve(points, middle, &right, tolerance_squared);
        }
    }

    pub fn from_arc(
        points: &mut Vec<Point>,
        start_point: Point,
        arc: &Arc,
        tolerance: &ToleranceSquared,
    ) {
        if start_point.distance_squared(&arc.mid_point()) <= **tolerance {
            points.push(arc.end_point())
        } else {
            let ((start, left), (middle, right)) = arc.subdivide(start_point);
            Self::from_arc(points, start, &left, tolerance);
            Self::from_arc(points, middle, &right, tolerance);
        }
    }
}
