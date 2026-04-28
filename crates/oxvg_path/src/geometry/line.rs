use crate::geometry::Point;

#[derive(Clone, Copy, PartialEq)]
/// A line is a set of two terminal points.
pub struct Line(pub [Point; 2]);

#[derive(Debug, PartialEq)]
pub enum Intersection {
    None,
    Intersection(Point),
    Parallel(Point, Point),
}

impl std::fmt::Debug for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "LINE({:?} {:?},{:?} {:?})",
            self.start().x(),
            self.start().y(),
            self.end().x(),
            self.end().y()
        ))
    }
}

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

    pub const fn left(&self) -> &Point {
        self.start().leftmost(self.end())
    }

    pub const fn right(&self) -> &Point {
        self.start().rightmost(self.end())
    }

    pub fn is_vertical(&self) -> bool {
        self.start().x() == self.end().x()
    }

    pub fn is_horizontal(&self) -> bool {
        self.start().y() == self.end().y()
    }

    /// Gets the point at which two lines cross.
    pub const fn intersection(&self, other: &Self) -> Intersection {
        let denom = self.denom(other);
        if denom == 0.0 {
            let cross = Point::cross(*self.start(), *self.end(), *other.start());
            if cross != 0.0 {
                return Intersection::None;
            }
            let overlap_left = self.left().rightmost(other.left());
            let overlap_right = self.right().leftmost(other.right());
            if overlap_left.x() > overlap_right.x() {
                return Intersection::None;
            }
            return Intersection::Parallel(*overlap_left, *overlap_right);
        }

        let self_normal = self.normal();
        let other_normal = other.normal();
        let self_constant = self.constant();
        let other_constant = other.constant();
        let cross = Point([
            (self_normal.y() * other_constant - other_normal.y() * self_constant) / denom,
            (other_normal.x() * self_constant - self_normal.x() * other_constant) / denom,
        ]);
        if cross.is_nan() || !cross.is_finite() {
            Intersection::None
        } else {
            Intersection::Intersection(cross)
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

#[cfg(test)]
mod test {
    use crate::geometry::{line::Intersection, Line, Point};

    #[test]
    fn intersection_some() {
        assert_eq!(
            Line([Point([-1.0, 0.0]), Point([1.0, 0.0])])
                .intersection(&Line([Point([0.0, -1.0]), Point([0.0, 1.0])])),
            Intersection::Intersection(Point::ZERO)
        );
    }
    #[test]
    fn intersection_none() {
        assert_eq!(
            Line([Point::ZERO, Point::UNIT])
                .intersection(&Line([Point([0.0, 1.0]), Point([1.0, 2.0])])),
            Intersection::None
        );
    }
    #[test]
    fn intersection_none_parallel() {
        assert_eq!(
            Line([Point([5.0, 5.0]), Point([15.0, 5.0])])
                .intersection(&Line([Point([0.0, 10.0]), Point([5.0, 10.0])])),
            Intersection::None
        );
    }
    #[test]
    fn intersection_parallel() {
        assert_eq!(
            Line([Point::splat(-2.0), Point::splat(1.0)])
                .intersection(&Line([Point::splat(-1.0), Point::splat(2.0)])),
            Intersection::Parallel(Point::splat(-1.0), Point::splat(1.0))
        );
    }
    #[test]
    fn intersection_parallel_opposed() {
        assert_eq!(
            Line([Point::splat(1.0), Point::splat(-2.0)])
                .intersection(&Line([Point::splat(-1.0), Point::splat(2.0)])),
            Intersection::Parallel(Point::splat(-1.0), Point::splat(1.0))
        );
    }
}
