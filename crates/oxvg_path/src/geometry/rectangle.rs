use std::ops::{Add, Sub};

use crate::geometry::Point;

/// A bounded 2D quadrilateral whose area is defined by minimum and maximum [`Point`].
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Rectangle([Point; 2]);

impl Sub<Point> for Rectangle {
    type Output = Rectangle;

    fn sub(self, rhs: Point) -> Self::Output {
        Self([self.min() - &rhs, self.max() - &rhs])
    }
}

impl Add<Point> for Rectangle {
    type Output = Rectangle;

    fn add(self, rhs: Point) -> Self::Output {
        Self([self.min() + &rhs, self.max() + &rhs])
    }
}

impl Rectangle {
    /// A rectangle spanning from negative to positive infinity
    pub const INFINITY: Self = Self([Point::NEG_INFINITY, Point::INFINITY]);
    /// A zero-dimension rectangle at (0, 0)
    pub const ZERO: Self = Self([Point::ZERO, Point::ZERO]);
    /// A rectangle bounding the unit vector
    pub const UNIT: Self = Self([Point::ZERO, Point::UNIT]);

    /// Returns a rectangle covering the minimum and maximum of the given terminals
    pub fn new(a: Point, b: Point) -> Self {
        Self([
            Point([a.x().min(b.x()), a.y().min(b.y())]),
            Point([a.x().max(b.x()), a.y().max(b.y())]),
        ])
    }

    /// Creates a rectangle assuming the minimum and maximum corners are valid
    pub fn new_unchecked(min: Point, max: Point) -> Self {
        Self([min, max])
    }

    fn valid(&self) -> bool {
        self.min_unchecked().x() <= self.max_unchecked().x()
            && self.min_unchecked().y() <= self.max_unchecked().y()
    }

    #[inline(always)]
    /// Returns the minimum point of the rectangle
    ///
    /// No debug assertions
    pub fn min_unchecked(&self) -> &Point {
        &self.0[0]
    }

    /// Returns the minimum point of the rectangle
    pub fn min(&self) -> &Point {
        debug_assert!(self.valid());
        self.min_unchecked()
    }

    /// Returns the maximum point of the rectangle
    ///
    /// No debug assertions
    pub fn max_unchecked(&self) -> &Point {
        &self.0[1]
    }

    /// Returns the maximum point of the rectangle
    pub fn max(&self) -> &Point {
        debug_assert!(self.valid());
        self.max_unchecked()
    }

    /// Returns the rectangle that fits within the two rectangles
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let result = Self([
            Point([
                self.min().x().max(other.min().x()),
                self.min().y().max(other.min().y()),
            ]),
            Point([
                self.max().x().min(other.max().x()),
                self.max().y().min(other.max().y()),
            ]),
        ]);
        if result.valid() {
            Some(result)
        } else {
            None
        }
    }

    /// Returns whether the two rectangle overlap each other
    pub fn intersects(&self, other: &Self) -> bool {
        self.contains(other.min())
    }

    /// Returns whether the rectangle contains the given point
    pub fn contains(&self, point: &Point) -> bool {
        self.min().x() <= point.x()
            && point.x() <= self.max().x()
            && self.min().y() <= point.y()
            && point.y() <= self.max().y()
    }

    /// Returns a point clamped within the bounds of the rectangle
    pub fn clamp(&self, point: &Point) -> Point {
        Point([
            point.x().clamp(self.min().x(), self.max().x()),
            point.y().clamp(self.min().y(), self.max().y()),
        ])
    }
}
