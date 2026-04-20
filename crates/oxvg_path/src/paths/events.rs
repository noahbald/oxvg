use itertools::Itertools as _;

use crate::{
    geometry::{Arc, Curve, Line, Point},
    paths::segment::ToleranceSquared,
};

pub enum Data {
    Line(Line),
    Curve(Curve, Vec<Line>),
    Arc(Arc, Vec<Line>),
}

impl Data {
    pub fn for_each<'a, F>(&'a self, mut f: F)
    where
        F: FnMut(&'a Line),
    {
        match self {
            Self::Line(p) => f(p),
            Self::Curve(_, p) | Self::Arc(_, p) => p.iter().for_each(f),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Line(_) => 1,
            Self::Curve(_, p) | Self::Arc(_, p) => p.len(),
        }
    }
}

pub struct Ring {
    pub start: Point,
    pub data: Vec<Data>,
}

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
                            cursor = curve.end_point();
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
                            Polygon::from_arc(&mut points, cursor, arc, tolerance_squared);
                            cursor = arc.end_point();
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
        }
    }
}

impl Path {
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
