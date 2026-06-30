use std::ops::Deref;

use geo::{Coord, LineString};
use i_float::float::compatible::FloatPointCompatible;

use crate::{geometry::Point, paths::segment};

#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    index: usize,
    source_start: Point,
    source: &'a segment::Data,
}

#[derive(Clone, Copy, Debug)]
pub struct BoolOpsCoord<'a> {
    coord: Coord<f64>,
    source: Option<Source<'a>>,
}

impl Deref for BoolOpsCoord<'_> {
    type Target = Coord<f64>;

    fn deref(&self) -> &Self::Target {
        &self.coord
    }
}

impl BoolOpsCoord<'_> {
    pub fn line_string(string: &[Self]) -> LineString<f64> {
        LineString(string.into_iter().map(Deref::deref).copied().collect())
    }
}

impl<'a> FloatPointCompatible for BoolOpsCoord<'a> {
    type Scalar = f64;

    fn from_xy(x: f64, y: f64) -> Self {
        Self {
            coord: Coord { x, y },
            source: None,
        }
    }

    fn x(&self) -> f64 {
        self.coord.x
    }

    fn y(&self) -> f64 {
        self.coord.y
    }
}

pub mod convert {
    use geo::{Coord, CoordsIter};

    use crate::{
        geometry::Point,
        paths::segment::{self, ToleranceSquared},
    };

    use super::{BoolOpsCoord, Source};

    pub fn ring_to_shape_path<'a>(segment: &'a segment::Segment) -> Vec<BoolOpsCoord<'a>> {
        if segment.data.is_empty() {
            return vec![];
        };

        let mut cursor = segment.start;
        std::iter::once(vec![BoolOpsCoord {
            coord: *cursor,
            source: segment.data.first().map(|source| Source {
                index: 0,
                source_start: cursor,
                source,
            }),
        }])
        .chain(
            segment
                .data
                .iter()
                .enumerate()
                .map(|(index, data)| match data {
                    segment::Data::LineTo(coord) => {
                        let start = cursor;
                        cursor = *coord;
                        vec![BoolOpsCoord {
                            coord: **coord,
                            source: Some(Source {
                                index,
                                source_start: start,
                                source: data,
                            }),
                        }]
                    }
                    segment::Data::CurveTo(curve) => {
                        let start = cursor;
                        cursor = curve.end_point;
                        curve
                            .with_start(start)
                            .coords_iter()
                            .skip(1)
                            .map(|coord| BoolOpsCoord {
                                coord,
                                source: Some(Source {
                                    index,
                                    source_start: start,
                                    source: data,
                                }),
                            })
                            .collect()
                    }
                    segment::Data::ArcTo(arc) => {
                        let start = cursor;
                        cursor = arc.end_point();
                        arc.coords_iter()
                            .skip(1)
                            .map(|coord| BoolOpsCoord {
                                coord,
                                source: Some(Source {
                                    index,
                                    source_start: start,
                                    source: data,
                                }),
                            })
                            .collect()
                    }
                }),
        )
        .flatten()
        .collect()
    }

    pub fn segment_path_from_shapes(shapes: Vec<Vec<Vec<BoolOpsCoord<'_>>>>) -> segment::Path {
        segment::Path(shapes.into_iter().flat_map(segment_from_shape).collect())
    }

    fn segment_from_shape(
        shape: Vec<Vec<BoolOpsCoord<'_>>>,
    ) -> impl Iterator<Item = segment::Segment> + use<'_> {
        shape.into_iter().map(segment_from_ring)
    }

    fn segment_from_ring(ring: Vec<BoolOpsCoord<'_>>) -> segment::Segment {
        let Some(first) = ring.first() else {
            return segment::Segment::empty(Point::default());
        };

        let tolerance = &ToleranceSquared(1e-6);
        let cursor = Point(first.coord);
        let mut segment = segment::Segment {
            start: cursor,
            data: vec![],
            closed: true,
        };

        let slice = |start: (Coord<f64>, Source), end: (Coord<f64>, Source)| match start.1.source {
            segment::Data::LineTo(_) => segment::Data::LineTo(Point(end.0)),
            segment::Data::CurveTo(curve) => {
                let t1 = Point(start.0);
                let t2 = Point(end.0);
                segment::Data::CurveTo(
                    if start.1.index < end.1.index {
                        curve.clamp_at(start.1.source_start, t1, t2, tolerance)
                    } else {
                        curve.reverse(start.1.source_start).clamp_at(
                            start.1.source_start,
                            t1,
                            t2,
                            tolerance,
                        )
                    }
                    .unwrap(),
                )
            }
            segment::Data::ArcTo(arc) => {
                let t1 = Point(start.0);
                let t2 = Point(end.0);
                segment::Data::ArcTo(
                    if start.1.index < end.1.index {
                        arc.clamp_at(t1, t2, tolerance)
                    } else {
                        arc.reverse().clamp_at(t1, t2, tolerance)
                    }
                    .unwrap(),
                )
            }
        };

        let mut current_start = None;
        let mut current_end = None;
        for (i, coord) in ring.into_iter().enumerate() {
            dbg!(coord);
            debug_assert!(coord.source.is_some());
            if let Some(source) = coord.source {
                let Some((_, _, start)) = current_start else {
                    current_start = Some((i, coord.coord, source));
                    current_end = current_start;
                    continue;
                };
                if std::ptr::eq(source.source, start.source) {
                    current_end = Some((i, coord.coord, source));
                    continue;
                }
            }

            // At this point, either
            // - coord is unsourced
            // - sources switch
            //
            // So
            // - build from current_start to current_end
            // - push unsourced coord
            if let Some((_, start_coord, start)) = current_start {
                let (_, end_coord, end) = current_end.expect("end must be set with start");
                debug_assert!(std::ptr::eq(start.source, end.source));

                let data = slice((start_coord, start), (end_coord, end));
                segment.data.push(data);
                current_start = None;
                current_end = None;
            }
            if coord.source.is_none() {
                segment.data.push(segment::Data::LineTo(Point(coord.coord)))
            }
        }
        if let Some((_, start_coord, start)) = current_start {
            let (_, end_coord, end) = current_end.expect("end must be set with start");
            debug_assert!(std::ptr::eq(start.source, end.source));

            let data = slice((start_coord, start), (end_coord, end));
            segment.data.push(data);
        }

        segment
    }
}
