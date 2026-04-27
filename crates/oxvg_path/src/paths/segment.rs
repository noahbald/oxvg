//! Path data represented as basic geometry.
//!
//! Using segments allows efficient use of more complicated operations, including
//!
//! - Simplification of paths and segments
//! - Boolean operations of paths and segments
//! - Translations of paths and segments
use std::ops::Deref;

use crate::geometry::{Arc, Curve, Point};

#[cfg(feature = "boolean")]
mod boolean;
mod convert;
mod simplify;

/// Tolerance for converting between SVG, Segments, and Polygons
pub struct Tolerance {
    /// The level of tolerance when comparing the error between distances
    pub positional: f64,
    /// The level of tolerance when comparing the error between angles
    pub angular: f64,
}

impl Default for Tolerance {
    fn default() -> Self {
        // TODO: Experiment for best defaults
        Self {
            positional: 1e-3,
            angular: 1e-3,
        }
    }
}

impl Tolerance {
    pub fn square(&self) -> ToleranceSquared {
        ToleranceSquared(self.positional * self.positional)
    }
}

pub struct ToleranceSquared(pub f64);

impl Deref for ToleranceSquared {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A reduced representation of an SVG path command
pub enum Data {
    /// A line commend
    LineTo(Point),
    /// A bezier command
    CurveTo(Curve),
    /// An arc command
    ArcTo(Arc),
}

#[derive(Debug, PartialEq)]
/// A segment represents some contiguous shape made from a set of commands
pub struct Segment {
    pub(crate) start: Point,
    pub(crate) data: Vec<Data>,
    pub(crate) closed: bool,
}

#[derive(Debug, PartialEq)]
/// A segment path is a set of disjointed shaped, each composed of a set of commands
pub struct Path(pub Vec<Segment>);

impl Data {
    pub fn end_point(&self) -> Point {
        match self {
            Self::LineTo(point) => *point,
            Self::CurveTo(curve) => curve.end_point(),
            Self::ArcTo(arc) => arc.end_point(),
        }
    }

    pub fn reverse(&self, start: Point) -> Self {
        match self {
            Data::LineTo(_) => Data::LineTo(start),
            Data::CurveTo(curve) => Data::CurveTo(Curve::new(
                curve.end_control(),
                curve.start_control(),
                start,
            )),
            Data::ArcTo(arc) => Data::ArcTo(Arc::new(
                arc.center(),
                arc.radii(),
                arc.start_angle() + arc.sweep_angle(),
                -arc.sweep_angle(),
                arc.x_rotation(),
            )),
        }
    }
}

impl Segment {
    pub fn start(&self) -> &Point {
        &self.start
    }

    pub fn data(&self) -> &[Data] {
        &self.data
    }

    pub fn closed(&self) -> bool {
        self.closed
    }
}

impl Path {
    pub fn close_segments(&mut self) {
        for segment in self.0.iter_mut().filter(|s| !s.closed) {
            segment.data.push(Data::LineTo(segment.start));
            segment.closed = true
        }
    }

    pub fn iter_start_cursor(&self) -> IterStartCursor {
        IterStartCursor {
            path: self,
            segment: 0,
            command: 0,
            cursor: Point::ZERO,
        }
    }

    pub fn iter_start_cursor_mut(&mut self) -> IterStartCursorMut {
        IterStartCursorMut {
            path: self,
            segment: 0,
            command: 0,
            cursor: Point::ZERO,
        }
    }
}

struct IterStartCursor<'a> {
    path: &'a Path,
    segment: usize,
    command: usize,
    cursor: Point,
}
struct IterStartCursorMut<'a> {
    path: &'a mut Path,
    segment: usize,
    command: usize,
    cursor: Point,
}
impl<'a> Iterator for IterStartCursor<'a> {
    type Item = (Point, &'a Data);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let segment = self.path.0.get(self.segment)?;
            if self.command == 0 {
                self.cursor = segment.start;
            }
            if let Some(data) = segment.data.get(self.command) {
                self.command += 1;
                let cursor = self.cursor;
                self.cursor = data.end_point();
                return Some((cursor, data));
            } else {
                self.segment += 1;
                self.command = 0;
            }
        }
    }
}
impl<'a> IterStartCursorMut<'a> {
    fn next(&mut self) -> Option<(Point, &mut Data)> {
        for segment in self.path.0.iter_mut().skip(self.segment) {
            if self.command == 0 {
                self.cursor = segment.start;
            }
            if let Some(data) = segment.data.get_mut(self.command) {
                self.command += 1;
                let cursor = self.cursor;
                self.cursor = data.end_point();
                return Some((cursor, data));
            } else {
                self.segment += 1;
                self.command = 0;
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use crate::{
        geometry::Point,
        paths::segment::{Data, Path, Segment},
    };

    #[test]
    fn close_segments() {
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![
                Data::LineTo(Point([0.0, 1.0])),
                Data::LineTo(Point([1.0, 1.0])),
            ],
            closed: false,
        }]);
        path.close_segments();

        assert_eq!(
            path,
            Path(vec![Segment {
                start: Point::ZERO,
                data: vec![
                    Data::LineTo(Point([0.0, 1.0])),
                    Data::LineTo(Point([1.0, 1.0])),
                    Data::LineTo(Point::ZERO),
                ],
                closed: true,
            }])
        );
    }
}
