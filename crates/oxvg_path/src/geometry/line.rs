//! Types for representing finite lines.
use std::ops::Deref;

use crate::geometry::{Point, Rectangle};

#[derive(Clone, Copy, PartialEq, Debug)]
/// A line is a set of two terminal points.
pub struct Line(pub geo_types::Line<f64>);

pub type Intersection = geo::LineIntersection<f64>;

impl Deref for Line {
    type Target = geo_types::Line<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Line {
    /// A zero length line at (0, 0)
    pub const ZERO: Self = Self::splat(0.0);
    /// A line equivalent to the unit vector
    pub const UNIT: Self = Self::new(Point::ZERO, Point::UNIT);
    /// A line spanning from (-INF, INF)
    pub const INFINITY: Self = Self::new(Point::NEG_INFINITY, Point::INFINITY);
    /// A line spanning from (NAN, NAN)
    pub const NAN: Self = Self::new(Point::NAN, Point::NAN);

    /// Returns a line with the given start and end points.
    pub const fn new(start: Point, end: Point) -> Self {
        Line(geo_types::Line {
            start: start.0,
            end: end.0,
        })
    }

    /// Returns a zero-length line with the same x/y value.
    pub const fn splat(n: f64) -> Self {
        Line::new(Point::splat(n), Point::splat(n))
    }

    /// Returns the starting point.
    pub const fn start(&self) -> Point {
        Point(self.0.start)
    }

    /// Returns the ending point.
    pub const fn end(&self) -> Point {
        Point(self.0.end)
    }

    /// Returns the vector of the line. A point from `[0, 0]`
    pub fn vector(&self) -> Point {
        self.end() - self.start()
    }

    /// Returns the length of the line.
    pub fn len(&self) -> f64 {
        self.vector().len()
    }

    /// Returns the leftmost point of the line
    pub fn left(&self) -> Point {
        self.start().leftmost(self.end())
    }

    /// Returns the rightmost point of the line
    pub fn right(&self) -> Point {
        self.start().rightmost(self.end())
    }

    /// Returns whether the line's ends are lie on the same x coordinate
    pub fn is_vertical(&self) -> bool {
        self.start().x == self.end().x
    }

    /// Returns whether the line's ends are lie on the same y coordinate
    pub fn is_horizontal(&self) -> bool {
        self.start().y == self.end().y
    }

    /// Returns a bounding box for the given line
    pub fn bounds(&self) -> Rectangle {
        Rectangle::new(self.start(), self.end())
    }

    pub fn distance_squared(&self, point: Point) -> f64 {
        use rstar::PointDistance as _;
        self.0.distance_2(&geo_types::Point(*point))
    }

    /// Gets the point at which two lines cross.
    pub fn intersection(&self, other: &Self) -> Option<Intersection> {
        geo::line_intersection::line_intersection(**self, **other)
    }

    /// Returns the gradient constant of the line
    pub fn constant(&self) -> f64 {
        self.start().x * self.end().y - self.end().x * self.start().y
    }

    /// Returns the normal vector of the line
    pub fn normal(&self) -> Point {
        Point::new(self.start().y - self.end().y, self.end().x - self.start().x)
    }

    /// Returns the denominator of two lines
    pub fn denom(&self, other: &Self) -> f64 {
        let a = self.normal();
        let b = other.normal();
        a.x * b.y - a.y * b.x
    }

    /// Returns the midpoint between the two ends of the line
    pub fn midpoint(&self) -> Point {
        self.start().midpoint(self.end())
    }
}
