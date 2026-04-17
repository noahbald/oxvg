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

#[derive(Debug, PartialEq)]
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
    start: Point,
    pub(crate) data: Vec<Data>,
    pub(crate) closed: bool,
}

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
