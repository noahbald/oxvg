//! Path types for polygonal representations of SVG path data.
use itertools::Itertools as _;

use crate::{
    geometry::{Arc, Curve, Line, Point},
    paths::segment::ToleranceSquared,
};

#[derive(Debug, Clone)]
/// A reduced representation of an SVG path command with equivalent polygons fitting along
/// the data up to some tolerance.
pub enum Data {
    /// A line command
    Line(Line),
    /// A bezier command
    Curve(Curve, Vec<Line>),
    /// A arc command
    Arc(Arc, Vec<Line>),
}

impl Data {
    /// Iterates along each point of the data's polygon.
    pub fn for_each<'a, F>(&'a self, mut f: F)
    where
        F: FnMut(&'a Line),
    {
        match self {
            Self::Line(p) => f(p),
            Self::Curve(_, p) | Self::Arc(_, p) => p.iter().for_each(f),
        }
    }

    /// Returns the number of point for the data's polygon.
    pub fn len(&self) -> usize {
        match self {
            Self::Line(_) => 1,
            Self::Curve(_, p) | Self::Arc(_, p) => p.len(),
        }
    }

    /// Returns whether the polygon has no points. Always `false`.
    pub fn is_empty(&self) -> bool {
        debug_assert!(self.len() > 0);
        false
    }

    /// Returns the end point.
    pub fn end_point(&self) -> Point {
        match self {
            Self::Line(p) => *p.end(),
            Self::Curve(p, _) => p.end_point(),
            Self::Arc(p, _) => p.end_point(),
        }
    }
}

#[derive(Debug, Clone)]
/// A ring is a closed polygon.
pub struct Ring {
    /// The start point of the ring.
    pub start: Point,
    /// The data and polygons of the ring.
    pub data: Vec<Data>,
    /// The interior rings of the polygon.
    pub interiors: Vec<Vec<Data>>,
}

#[derive(Debug, Clone)]
/// An event path is a set of closed polygons, each composed of a set of command-polygon pairs
pub struct Path(pub Vec<Ring>);

impl Ring {
    fn from_segment(
        value: &crate::paths::segment::Segment,
        tolerance_squared: &ToleranceSquared,
    ) -> Self {
        debug_assert!(value.closed);
        let mut cursor = value.start;
        Self {
            start: value.start,
            data: value
                .data
                .iter()
                .map(|data| {
                    use crate::geometry::Polygon;
                    use crate::paths::segment;
                    match data {
                        segment::Data::LineTo(point) => {
                            let edge = Line([cursor, *point]);
                            cursor = *point;
                            Data::Line(edge)
                        }
                        segment::Data::CurveTo(curve) => {
                            let mut points = vec![];
                            Polygon::from_curve(&mut points, cursor, curve, tolerance_squared);
                            cursor = *points.last().unwrap_or(&cursor);
                            Data::Curve(
                                *curve,
                                points
                                    .into_iter()
                                    .tuple_windows()
                                    .map(|(a, b)| Line([a, b]))
                                    .collect(),
                            )
                        }
                        segment::Data::ArcTo(arc) => {
                            let mut points = vec![];
                            Polygon::from_arc(&mut points, cursor, arc, tolerance_squared, 0);
                            cursor = *points.last().unwrap_or(&cursor);
                            Data::Arc(
                                *arc,
                                points
                                    .into_iter()
                                    .tuple_windows()
                                    .map(|(a, b)| Line([a, b]))
                                    .collect(),
                            )
                        }
                    }
                })
                .collect(),
            interiors: vec![],
        }
    }
}

impl Path {
    /// Returns an event path from a segmented path, assuming all segments are closed.
    pub fn from_segments(
        value: &crate::paths::segment::Path,
        tolerance_squared: &ToleranceSquared,
    ) -> Self {
        Self(
            value
                .0
                .iter()
                .map(|segment| Ring::from_segment(segment, tolerance_squared))
                .collect(),
        )
    }
}
