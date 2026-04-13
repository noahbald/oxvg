use crate::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
/// A line is a set of two terminal points.
pub struct Line(pub [Point; 2]);

impl Line {
    /// Returns the starting point.
    pub const fn start(&self) -> &Point {
        &self.0[0]
    }

    /// Returns the ending point.
    pub const fn end(&self) -> &Point {
        &self.0[1]
    }

    /// Returns the vector of the line. A point from `[0, 0]`
    pub fn vector(&self) -> Point {
        self.end() - self.start()
    }

    /// Returns the length of the line.
    pub fn len(&self) -> f64 {
        self.vector().len()
    }

    /// Gets the point at which two lines cross.
    pub const fn intersection(&self, other: &Self) -> Option<Point> {
        let denom = self.denom(other);
        if denom == 0.0 {
            return None;
        }

        let self_normal = self.normal();
        let other_normal = other.normal();
        let self_constant = self.constant();
        let other_constant = other.constant();
        let cross = Point([
            (self_normal.y() * other_constant - other_normal.y() * self_constant) / denom,
            (self_normal.x() * other_constant - other_normal.x() * self_constant) / -denom,
        ]);
        if cross.is_nan() || !cross.is_finite() {
            None
        } else {
            Some(cross)
        }
    }

    pub const fn constant(&self) -> f64 {
        self.start().x() * self.end().y() - self.end().x() * self.start().y()
    }

    pub const fn normal(&self) -> Point {
        Point([
            self.start().y() - self.end().y(),
            self.end().x() - self.start().x(),
        ])
    }

    pub const fn denom(&self, other: &Self) -> f64 {
        let a = self.normal();
        let b = other.normal();
        a.0[0] * b.0[1] - a.0[1] * b.0[0]
    }

    pub fn midpoint(&self) -> Point {
        self.start().midpoint(&self.end())
    }
}
