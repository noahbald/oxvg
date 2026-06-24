//! Path data represented as basic geometry.
//!
//! Using segments allows efficient use of more complicated operations, including
//!
//! - Simplification of paths and segments
//! - Boolean operations of paths and segments
//! - Translations of paths and segments
#[cfg(feature = "wasm")]
use tsify::Tsify;

use std::ops::Deref;

use crate::geometry::{Arc, Curve, Point};

#[cfg(feature = "boolean")]
mod boolean;
mod convert;
mod simplify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// Tolerance for converting between SVG, Segments, and Polygons
pub struct Tolerance {
    /// The level of tolerance when comparing the error between distances
    #[cfg_attr(feature = "serde", serde(default = "positional_default"))]
    pub positional: f64,
    /// The level of tolerance when comparing the error between angles
    #[cfg_attr(feature = "serde", serde(default = "angular_default"))]
    pub angular: f64,
    /// The number of decimal places to round numbers to during processing
    #[cfg_attr(feature = "serde", serde(default = "precision_default"))]
    pub precision: i32,
}

const fn positional_default() -> f64 {
    1e-3
}
const fn angular_default() -> f64 {
    1e-3
}
const fn precision_default() -> i32 {
    3
}

impl Default for Tolerance {
    fn default() -> Self {
        // TODO: Experiment for best defaults
        Self {
            positional: positional_default(),
            angular: angular_default(),
            precision: precision_default(),
        }
    }
}

impl Tolerance {
    /// Returns the square of the positional tolerance.
    pub fn square(&self) -> ToleranceSquared {
        ToleranceSquared(self.positional * self.positional)
    }

    /// Returns the scale for the precision.
    pub fn precision(&self) -> TolerancePrecision {
        TolerancePrecision(10.0_f64.powi(self.precision))
    }
}

/// A monad representing a squared positional tolerance.
pub struct ToleranceSquared(pub f64);

#[derive(Debug)]
/// A monad representing a scale for rounding a number to some precision
pub struct TolerancePrecision(pub f64);

impl Deref for ToleranceSquared {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TolerancePrecision {
    /// Expands the number to a rounded number
    pub const fn scale(&self, value: f64) -> f64 {
        (value * self.0).round()
    }

    /// Shrink the number to a decimal number
    pub const fn descale(&self, value: f64) -> f64 {
        value / self.0
    }

    /// Rounds a number to the given precision
    pub const fn round(&self, value: f64) -> f64 {
        self.descale(self.scale(value))
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A reduced representation of an SVG path command
pub enum Data {
    /// A line command
    LineTo(Point),
    /// A bezier command
    CurveTo(Curve),
    /// An arc command
    ArcTo(Arc),
}

#[derive(Debug, PartialEq, Clone)]
/// A segment represents some contiguous shape made from a set of commands
pub struct Segment {
    pub(crate) start: Point,
    pub(crate) data: Vec<Data>,
    pub(crate) closed: bool,
}

#[derive(Debug, PartialEq, Clone)]
/// A segment path is a set of disjointed shapes, each composed of a set of commands
pub struct Path(pub Vec<Segment>);

impl Data {
    /// Returns the end point of the data item.
    pub fn end_point(&self) -> Point {
        match self {
            Self::LineTo(point) => *point,
            Self::CurveTo(curve) => curve.end_point,
            Self::ArcTo(arc) => arc.end_point(),
        }
    }

    /// Returns the equivalent data item going from the end to the start.
    #[must_use]
    pub fn reverse(&self, start: Point) -> Self {
        match self {
            Data::LineTo(_) => Data::LineTo(start),
            Data::CurveTo(curve) => {
                Data::CurveTo(Curve::new(curve.end_control, curve.start_control, start))
            }
            Data::ArcTo(arc) => Data::ArcTo(
                Arc::new(
                    arc.center(),
                    arc.radii(),
                    arc.start_angle() + arc.sweep_angle(),
                    -arc.sweep_angle(),
                    arc.x_rotation(),
                )
                .with_end_point_memo(start),
            ),
        }
    }
}

impl Segment {
    pub fn new(start: Point, data: Vec<Data>, closed: bool) -> Self {
        Self {
            start,
            data,
            closed,
        }
    }

    pub fn empty(start: Point) -> Self {
        Self {
            start,
            data: vec![],
            closed: false,
        }
    }

    pub fn with_capacity(start: Point, capacity: usize) -> Self {
        Self {
            start,
            data: Vec::with_capacity(capacity),
            closed: false,
        }
    }

    pub fn push(&mut self, data: Data) {
        self.data.push(data);
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn unclose(&mut self) {
        self.closed = false;
    }

    /// Returns the start point of the segment.
    pub fn start(&self) -> &Point {
        &self.start
    }

    /// Returns the data of the segment.
    pub fn data(&self) -> &[Data] {
        &self.data
    }

    /// Returns whether the segment is closed.
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The end point of the segment's last command
    pub fn end_point(&self) -> Point {
        if self.closed {
            *self.start()
        } else if let Some(last) = self.data.last() {
            last.end_point()
        } else {
            *self.start()
        }
    }
}

impl Path {
    /// Closes unclosed segments within the path.
    pub fn close_segments(&mut self) {
        for segment in self.0.iter_mut().filter(|s| !s.closed) {
            segment.data.push(Data::LineTo(segment.start));
            segment.closed = true;
        }
    }

    /// Returns an iterator for all the data within the path with the star point of each data item with it.
    pub fn iter_start_cursor(&self) -> IterStartCursor<'_> {
        IterStartCursor {
            path: self,
            segment: 0,
            command: 0,
            cursor: Point::ZERO,
        }
    }

    /// Returns a mutable iterator for all the data within the path with the star point of each data item
    /// with it.
    pub fn iter_start_cursor_mut(&mut self) -> IterStartCursorMut<'_> {
        IterStartCursorMut {
            path: self,
            segment: 0,
            command: 0,
            cursor: Point::ZERO,
        }
    }
}

/// The data item of the path and it's context.
pub struct IterStartCursorItem<T> {
    /// The start point of the data item's segment
    pub segment_start: Point,
    /// The relative start point of the data item's segment
    pub segment_start_by: Point,
    /// The start point for the current data item
    pub cursor: Point,
    /// The data item. If the item is `None` then this item represents a standalone `M` segment
    pub data: Option<T>,
    /// The proceeding data item.
    pub next: Option<T>,
    /// The index of the data within the segment
    pub command: usize,
    /// Whether the segment is closed by a `Z` command
    pub close: bool,
}

/// An iterator for all the data within the path with the star point of each data item with it.
pub struct IterStartCursor<'a> {
    path: &'a Path,
    segment: usize,
    command: usize,
    cursor: Point,
}
/// A mutable iterator for all the data within the path with the star point of each data item with it.
pub struct IterStartCursorMut<'a> {
    path: &'a mut Path,
    segment: usize,
    command: usize,
    cursor: Point,
}
impl<'a> Iterator for IterStartCursor<'a> {
    type Item = IterStartCursorItem<&'a Data>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let segment = self.path.0.get(self.segment)?;
            let segment_start = segment.start;
            let segment_start_by = if self.segment > 0 {
                segment.start - self.path.0[self.segment - 1].end_point()
            } else {
                segment.start
            };
            if self.command == 0 {
                self.cursor = segment.start;
            }
            if let Some(data) = segment.data.get(self.command) {
                let command = self.command;
                self.command += 1;
                let cursor = self.cursor;
                self.cursor = data.end_point();
                return Some(IterStartCursorItem {
                    segment_start,
                    segment_start_by,
                    cursor,
                    data: Some(data),
                    next: segment.data.get(self.command),
                    command,
                    close: segment.closed && command == segment.data.len() - 1,
                });
            } else if self.command == 0 {
                self.segment += 1;
                return Some(IterStartCursorItem {
                    segment_start,
                    segment_start_by,
                    cursor: segment_start,
                    data: None,
                    next: None,
                    command: 0,
                    close: false,
                });
            }
            self.segment += 1;
            self.command = 0;
        }
    }
}
impl IterStartCursorMut<'_> {
    fn next(&mut self) -> Option<IterStartCursorItem<&mut Data>> {
        let mut last_segment_end = if self.segment > 0 {
            Some(self.path.0[self.segment - 1].end_point())
        } else {
            None
        };
        for segment in self.path.0.iter_mut().skip(self.segment) {
            if self.command == 0 {
                self.cursor = segment.start;
            }
            let segment_start = segment.start;
            let segment_start_by = segment_start - last_segment_end.unwrap_or_default();
            last_segment_end = Some(segment.end_point());
            let close = segment.closed && self.command == segment.data.len() - 1;
            if self.command < segment.data.len() {
                let (left, right) = segment.data.split_at_mut(self.command + 1);
                let data = left.last_mut().unwrap();
                let command = self.command;
                self.command += 1;
                let cursor = self.cursor;
                self.cursor = data.end_point();
                return Some(IterStartCursorItem {
                    segment_start,
                    segment_start_by,
                    cursor,
                    data: Some(data),
                    next: right.first_mut(),
                    command,
                    close,
                });
            } else if self.command == 0 {
                self.segment += 1;
                return Some(IterStartCursorItem {
                    segment_start,
                    segment_start_by,
                    cursor: segment_start,
                    data: None,
                    next: None,
                    command: 0,
                    close: false,
                });
            }
            self.segment += 1;
            self.command = 0;
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
                Data::LineTo(Point::new(0.0, 1.0)),
                Data::LineTo(Point::new(1.0, 1.0)),
            ],
            closed: false,
        }]);
        path.close_segments();

        assert_eq!(
            path,
            Path(vec![Segment {
                start: Point::ZERO,
                data: vec![
                    Data::LineTo(Point::new(0.0, 1.0)),
                    Data::LineTo(Point::new(1.0, 1.0)),
                    Data::LineTo(Point::ZERO),
                ],
                closed: true,
            }])
        );
    }
}
