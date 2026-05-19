use crate::{
    geometry::{Arc, Curve, Point},
    paths::segment::ToleranceSquared,
};

/// A polygon is a list of points
#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    /// The points making up the polygon
    pub points: Vec<Point>,
    /// Whether the polygon is closed
    pub closed: bool,
}

impl Polygon {
    /// Creates a polygon fitting the curve up to some tolerance
    pub fn from_curve(
        points: &mut Vec<Point>,
        start: Point,
        curve: &Curve,
        tolerance_squared: &ToleranceSquared,
    ) {
        let (start_control_distance, end_control_distance) =
            curve.control_point_distance_squared(start);

        if start_control_distance <= **tolerance_squared
            && end_control_distance <= **tolerance_squared
        {
            if points.is_empty() {
                points.push(start);
            }
            points.push(curve.end_point());
        } else {
            let (left, middle, right) = curve.subdivide(start);
            Self::from_curve(points, start, &left, tolerance_squared);
            Self::from_curve(points, middle, &right, tolerance_squared);
        }
    }

    /// Creates a polygon fitting the arc up to some tolerance
    pub fn from_arc(
        points: &mut Vec<Point>,
        start: Point,
        arc: &Arc,
        tolerance: &ToleranceSquared,
        depth: usize,
    ) {
        if start.distance_squared(&arc.mid_point()) <= **tolerance || depth >= 7 {
            if points.is_empty() {
                points.push(start);
            }
            points.push(arc.end_point())
        } else {
            let (left, right) = arc.subdivide();
            Self::from_arc(points, start, &left, tolerance, depth + 1);
            Self::from_arc(points, arc.mid_point(), &right, tolerance, depth + 1);
        }
    }
}
