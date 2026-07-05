use std::ops::Deref;

use geo::{Coord, LineString};
use i_float::float::compatible::FloatPointCompatible;
use rstar::{RTreeObject, AABB};

use crate::{
    geometry::{Point, Rectangle},
    paths::segment,
};

#[derive(Debug)]
pub struct Source<'a> {
    data: &'a segment::Data,
    start: Point,
    bbox: Rectangle,
    contour: Vec<Coord<f64>>,
}

#[derive(Clone, Copy, Debug)]
pub struct BoolOpsCoord(Coord<f64>);

struct RTreeEntry<'a> {
    envelope: AABB<[f64; 2]>,
    source: Source<'a>,
}

impl RTreeObject for RTreeEntry<'_> {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

impl Deref for BoolOpsCoord {
    type Target = Coord<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BoolOpsCoord {
    pub fn line_string(string: &[Self]) -> LineString<f64> {
        LineString(string.iter().map(Deref::deref).copied().collect())
    }
}

impl FloatPointCompatible for BoolOpsCoord {
    type Scalar = f64;

    fn from_xy(x: f64, y: f64) -> Self {
        Self(Coord { x, y })
    }

    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }
}

pub mod convert {
    use geo::{Coord, CoordsIter};
    use itertools::Itertools;
    use rstar::{RTree, RTreeObject, AABB};

    use crate::{
        geometry::{Line, Point, Rectangle, ToleranceSquared},
        paths::segment,
    };

    use super::{BoolOpsCoord, RTreeEntry, Source};

    pub fn flatten_segment<'a>(
        segment: &'a segment::Segment,
        registry: &mut Vec<Source<'a>>,
    ) -> Vec<BoolOpsCoord> {
        if segment.data.is_empty() {
            return vec![];
        }

        let mut coords = vec![];
        let mut cursor = segment.start;
        coords.push(BoolOpsCoord(*cursor));

        for data in &segment.data {
            let start = cursor;
            let mut segment_coords = vec![];

            match data {
                segment::Data::LineTo(coord) => {
                    cursor = *coord;
                    segment_coords.push(**coord);
                }
                segment::Data::CurveTo(curve) => {
                    cursor = curve.end_point;
                    segment_coords.extend(curve.with_start(start).coords_iter().skip(1));
                }
                segment::Data::ArcTo(arc) => {
                    cursor = arc.end_point();
                    segment_coords.extend(arc.coords_iter().skip(1));
                }
            }

            let bbox =
                Rectangle::from_coords(std::iter::once(&*start).chain(segment_coords.iter()));
            registry.push(Source {
                data,
                start,
                bbox,
                contour: std::iter::once(*start)
                    .chain(segment_coords.iter().copied())
                    .collect(),
            });
            coords.extend(segment_coords.into_iter().map(BoolOpsCoord));
        }

        coords
    }

    pub fn segment_path_from_shapes(
        shapes: Vec<Vec<Vec<BoolOpsCoord>>>,
        registry: Vec<Source<'_>>,
    ) -> segment::Path {
        let r_tree = rstar::RTree::bulk_load(
            registry
                .into_iter()
                .map(|source| {
                    let bbox = Rectangle::from_coords(source.contour.iter());
                    RTreeEntry {
                        envelope: AABB::from_corners(bbox.min().into(), bbox.max().into()),
                        source,
                    }
                })
                .collect(),
        );
        segment::Path(
            shapes
                .into_iter()
                .flat_map(|shape| {
                    shape
                        .into_iter()
                        .map(|ring| segment_from_ring(ring, &r_tree))
                })
                .collect(),
        )
    }

    fn segment_from_ring(ring: Vec<BoolOpsCoord>, r_tree: &RTree<RTreeEntry>) -> segment::Segment {
        enum Action<'a> {
            Original {
                seg: &'a Source<'a>,
                start: Coord,
                end: Coord,
            },
            Line {
                end: Coord,
            },
        }

        let Some(first) = ring.first() else {
            return segment::Segment::empty(Point::default());
        };

        let tolerance = ToleranceSquared(1e-6);
        let cursor = Point(**first);
        let mut segment = segment::Segment {
            start: cursor,
            data: vec![],
            closed: true,
        };

        let mut actions = vec![];

        for (a, b) in ring.into_iter().tuple_windows() {
            if let Some(seg) = find_matching_segment(*a, *b, r_tree) {
                let mut merged = false;

                if let Some(Action::Original {
                    seg: last_seg,
                    end: p_end,
                    ..
                }) = actions.last_mut()
                {
                    if std::ptr::eq(seg.data, last_seg.data) {
                        *p_end = *b;
                        merged = true;
                    }
                }
                if !merged {
                    actions.push(Action::Original {
                        seg,
                        start: *a,
                        end: *b,
                    });
                }
            } else {
                actions.push(Action::Line { end: *b });
            }
        }

        for action in actions {
            match action {
                Action::Original {
                    seg,
                    start: p_start,
                    end: p_end,
                } => {
                    let t1 = Point(p_start);
                    let t2 = Point(p_end);
                    match seg.data {
                        segment::Data::LineTo(_) => {
                            segment.data.push(segment::Data::LineTo(t2));
                        }
                        segment::Data::CurveTo(curve) => {
                            let t1 = curve.t_at(seg.start, t1, tolerance).unwrap();
                            let t2 = curve.t_at(seg.start, t2, tolerance).unwrap();
                            segment.data.push(segment::Data::CurveTo(if t1 <= t2 {
                                curve.clamp_t(seg.start, t1, t2)
                            } else {
                                let end = curve.end_point;
                                curve.reverse(seg.start).clamp_t(end, 1.0 - t1, 1.0 - t2)
                            }));
                        }
                        segment::Data::ArcTo(arc) => {
                            let t1 = arc.t_at(t1, tolerance).unwrap();
                            let t2 = arc.t_at(t2, tolerance).unwrap();
                            segment.data.push(segment::Data::ArcTo(if t1 <= t2 {
                                arc.clamp_t(t1, t2)
                            } else {
                                arc.reverse().clamp_t(1.0 - t1, 1.0 - t2)
                            }));
                        }
                    }
                }
                Action::Line { end: p_end } => {
                    segment.data.push(segment::Data::LineTo(Point(p_end)));
                }
            }
        }

        segment
    }

    fn find_matching_segment<'a, 'b>(
        a: Coord<f64>,
        b: Coord<f64>,
        r_tree: &'b RTree<RTreeEntry<'a>>,
    ) -> Option<&'b Source<'a>> {
        let mid = Point(a).midpoint(Point(b));
        let mut best_match = None;
        let mut min_total_distance = f64::MAX;

        let tolerance = 1e-3;
        let bbox = Rectangle::new(Point(a), Point(b));
        let query_envolope: AABB<[f64; 2]> = <RTreeEntry as RTreeObject>::Envelope::from_corners(
            (bbox.min() - *Point::splat(tolerance)).into(),
            (bbox.max() + *Point::splat(tolerance)).into(),
        );

        for entry in r_tree.locate_in_envelope_intersecting(&query_envolope) {
            let seg = &entry.source;
            if !seg.bbox.intersects(&bbox) {
                continue;
            }

            let dist_a = project_on_polyline(Point(a), &seg.contour);
            let dist_b = project_on_polyline(Point(b), &seg.contour);
            let dist_m = project_on_polyline(mid, &seg.contour);

            if dist_a < tolerance && dist_b < tolerance && dist_m < tolerance {
                let total_dist = dist_a + dist_b + dist_m;
                if total_dist < min_total_distance {
                    min_total_distance = total_dist;
                    best_match = Some(seg);
                }
            }
        }
        best_match
    }

    fn project_on_polyline(p: Point, polyline: &[Coord<f64>]) -> f64 {
        let mut min_dist_squared = f64::MAX;

        for (p0, p1) in polyline.iter().tuple_windows() {
            let segment = Line::new(Point(*p0), Point(*p1));
            let dist = segment.distance_squared(p);

            if dist < min_dist_squared {
                min_dist_squared = dist;
            }
        }

        min_dist_squared.sqrt()
    }
}
