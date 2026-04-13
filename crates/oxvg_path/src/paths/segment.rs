//! Path data represented as basic geometry.
//!
//! Using segments allows efficient use of more complicated operations, including
//!
//! - Simplification of paths and segments
//! - Boolean operations of paths and segments
//! - Translations of paths and segments
use crate::geometry::{Arc, Curve, Point};

mod boolean;
mod convert;
mod simplify;

/// Use `CurveError` for tolerance
pub const DEFAULT_TOLERANCE: f64 = 1e-6;

#[derive(Debug, PartialEq)]
/// A reduced representation of an SVG path command
pub enum Data {
    /// A line commend
    LineTo(Point),
    /// A bezier command
    QuadTo(Curve),
    /// An arc command
    ArcTo(Arc),
}

#[derive(Debug, PartialEq)]
/// A segment represents some contiguous shape made from a set of commands
pub struct Segment {
    start: Point,
    pub(crate) data: Vec<Data>,
    closed: bool,
}

/// A segment path is a set of disjointed shaped, each composed of a set of commands
pub struct Path(pub Vec<Segment>);

impl Data {
    pub fn end_point(&self) -> Point {
        match self {
            Self::LineTo(point) => *point,
            Self::QuadTo(curve) => curve.end_point(),
            Self::ArcTo(arc) => arc.end_point(),
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
