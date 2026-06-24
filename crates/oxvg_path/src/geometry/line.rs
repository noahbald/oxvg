//! Types for representing finite lines.
use crate::geometry::{Point, Rectangle};

#[derive(Clone, Copy, PartialEq, Debug)]
/// A line is a set of two terminal points.
pub struct Line(pub geo_types::Line<f64>);

#[derive(Debug, PartialEq)]
/// A result for an intersection between two lines.
pub enum Intersection {
    /// The line does not intersect.
    None,
    /// The line intersects at a single point.
    Intersection(Point),
    /// The lines are parallel and intersect at infitite points between two terminals.
    Parallel(Rectangle),
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

    /// Gets the point at which two lines cross.
    pub fn intersection(&self, other: &Self) -> Intersection {
        let Some(bounds) = self.bounds().intersection(&other.bounds()) else {
            return Intersection::None;
        };

        let va = self.vector();
        let vb = other.vector();
        let e = other.start() - self.start();

        let mut cross = Point::ZERO.cross(va, vb);
        if cross != 0.0 {
            let s = Point::ZERO.cross(e, vb) / cross;
            if !(0.0..=1.0).contains(&s) {
                return Intersection::None;
            }
            let t = Point::ZERO.cross(e, va) / cross;
            if !(0.0..=1.0).contains(&t) {
                return Intersection::None;
            }
            let p = if s == 0.0 || s == 1.0 {
                self.start().lerp(self.end(), s)
            } else if t == 0.0 || t == 1.0 {
                other.start().lerp(other.end(), t)
            } else {
                self.start().lerp(self.end(), s)
            };
            return Intersection::Intersection(bounds.clamp(p));
        }

        cross = Point::ZERO.cross(e, va);
        if cross != 0.0 {
            return Intersection::None;
        }

        let sqr_len_a = va.dot(va);
        let sa = va.dot(e) / sqr_len_a;
        let sb = sa + va.dot(vb) / sqr_len_a;
        let smin = sa.min(sb);
        let smax = sa.max(sb);

        if smin <= 1.0 && smax >= 0.0 {
            if smin == 1.0 {
                Intersection::Intersection(bounds.clamp(self.start().lerp(self.end(), smin)))
            } else if smax == 0.0 {
                Intersection::Intersection(bounds.clamp(self.start().lerp(self.end(), smax)))
            } else {
                Intersection::Parallel(Rectangle::new(
                    bounds.clamp(self.start().lerp(self.end(), smin.max(0.0))),
                    bounds.clamp(self.start().lerp(self.end(), smax.min(1.0))),
                ))
            }
        } else {
            Intersection::None
        }
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

#[cfg(test)]
mod test {
    use crate::geometry::{line::Intersection, Line, Point, Rectangle};

    #[test]
    fn intersection_some() {
        assert_eq!(
            Line::new(Point::new(-1.0, 0.0), Point::new(1.0, 0.0))
                .intersection(&Line::new(Point::new(0.0, -1.0), Point::new(0.0, 1.0))),
            Intersection::Intersection(Point::ZERO)
        );
    }
    #[test]
    fn intersection_none() {
        assert_eq!(
            Line::new(Point::ZERO, Point::UNIT)
                .intersection(&Line::new(Point::new(0.0, 1.0), Point::new(1.0, 2.0))),
            Intersection::None
        );
    }
    #[test]
    fn intersection_none_parallel() {
        assert_eq!(
            Line::new(Point::new(5.0, 5.0), Point::new(15.0, 5.0))
                .intersection(&Line::new(Point::new(0.0, 10.0), Point::new(5.0, 10.0))),
            Intersection::None
        );
    }
    #[test]
    fn intersection_parallel() {
        assert_eq!(
            Line::new(Point::splat(-2.0), Point::splat(1.0))
                .intersection(&Line::new(Point::splat(-1.0), Point::splat(2.0))),
            Intersection::Parallel(Rectangle::new(Point::splat(-1.0), Point::splat(1.0)))
        );
    }
    #[test]
    fn intersection_parallel_opposed() {
        assert_eq!(
            Line::new(Point::splat(1.0), Point::splat(-2.0))
                .intersection(&Line::new(Point::splat(-1.0), Point::splat(2.0))),
            Intersection::Parallel(Rectangle::new(Point::splat(-1.0), Point::splat(1.0)))
        );
    }

    #[test]
    fn intersection_variety() {
        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(1.0, 0.0), Point::new(2.0, 2.0))),
            Intersection::None
        );
        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(1.0, 0.0), Point::new(10.0, 2.0))),
            Intersection::None
        );
        assert_eq!(
            Line::new(Point::new(2.0, 2.0), Point::new(3.0, 3.0),)
                .intersection(&Line::new(Point::new(0.0, 6.0), Point::new(2.0, 4.0))),
            Intersection::None
        );

        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(1.0, 0.0), Point::new(0.0, 1.0))),
            Intersection::Intersection(Point::new(0.5, 0.5))
        );

        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(0.0, 1.0), Point::new(0.0, 0.0))),
            Intersection::Intersection(Point::new(0.0, 0.0))
        );
        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(0.0, 1.0), Point::new(1.0, 1.0))),
            Intersection::Intersection(Point::new(1.0, 1.0))
        );

        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(0.5, 0.5), Point::new(1.0, 0.0))),
            Intersection::Intersection(Point::new(0.5, 0.5))
        );

        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0),)
                .intersection(&Line::new(Point::new(1.0, 1.0), Point::new(5.0, 5.0))),
            Intersection::Parallel(Rectangle::new(Point::new(1.0, 1.0), Point::new(5.0, 5.0)))
        );
        assert_eq!(
            Line::new(Point::new(1.0, 1.0), Point::new(10.0, 10.0),)
                .intersection(&Line::new(Point::new(1.0, 1.0), Point::new(5.0, 5.0))),
            Intersection::Parallel(Rectangle::new(Point::new(1.0, 1.0), Point::new(5.0, 5.0)))
        );
        assert_eq!(
            Line::new(Point::new(3.0, 3.0), Point::new(10.0, 10.0),)
                .intersection(&Line::new(Point::new(0.0, 0.0), Point::new(5.0, 5.0))),
            Intersection::Parallel(Rectangle::new(Point::new(3.0, 3.0), Point::new(5.0, 5.0)))
        );
        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))),
            Intersection::Parallel(Rectangle::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0)))
        );
        assert_eq!(
            Line::new(Point::new(1.0, 1.0), Point::new(0.0, 0.0),)
                .intersection(&Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))),
            Intersection::Parallel(Rectangle::new(Point::new(1.0, 1.0), Point::new(0.0, 0.0)))
        );

        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(1.0, 1.0), Point::new(2.0, 2.0))),
            Intersection::Intersection(Point::new(1.0, 1.0))
        );
        assert_eq!(
            Line::new(Point::new(1.0, 1.0), Point::new(0.0, 0.0),)
                .intersection(&Line::new(Point::new(1.0, 1.0), Point::new(2.0, 2.0))),
            Intersection::Intersection(Point::new(1.0, 1.0))
        );
        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(2.0, 2.0), Point::new(4.0, 4.0))),
            Intersection::None
        );
        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0),)
                .intersection(&Line::new(Point::new(0.0, -1.0), Point::new(1.0, 0.0))),
            Intersection::None
        );
        assert_eq!(
            Line::new(Point::new(1.0, 1.0), Point::new(0.0, 0.0),)
                .intersection(&Line::new(Point::new(0.0, -1.0), Point::new(1.0, 0.0))),
            Intersection::None
        );
        assert_eq!(
            Line::new(Point::new(0.0, -1.0), Point::new(1.0, 0.0),)
                .intersection(&Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))),
            Intersection::None
        );

        assert_eq!(
            Line::new(Point::new(0.0, 0.5), Point::new(1.0, 1.5),)
                .intersection(&Line::new(Point::new(0.0, 1.0), Point::new(1.0, 0.0))),
            Intersection::Intersection(Point::new(0.25, 0.75))
        );

        assert_eq!(
            Line::new(Point::new(0.0, 0.0), Point::new(1.0, 0.0),)
                .intersection(&Line::new(Point::new(1.0, -1.0), Point::new(2.0, 1.0))),
            Intersection::None
        );
    }
}
