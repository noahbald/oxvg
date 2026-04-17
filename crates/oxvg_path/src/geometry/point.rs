use std::{
    f64::consts::PI,
    ops::{Add, Div, Mul, Sub},
};

use crate::geometry::{Curve, Line};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
/// A point is an `[x, y]` coordinate. Points are the atomic unit of geometry.
pub struct Point(pub [f64; 2]);

#[derive(Debug, PartialEq)]
/// The quadrant of a unit circle.
pub enum Quadrant {
    /// Between 0 and 90 degrees
    A,
    /// Between 90 and 180 degrees
    B,
    /// Between 180 and 270 degrees
    C,
    /// Between 270 and 360 degrees
    D,
}

impl Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self([self.x() + rhs.x(), self.y() + rhs.y()])
    }
}
impl Add for &Point {
    type Output = Point;

    fn add(self, rhs: Self) -> Self::Output {
        Point::add(*self, *rhs)
    }
}
impl Add<f64> for Point {
    type Output = Point;

    fn add(self, rhs: f64) -> Self::Output {
        Self([self.x() + rhs, self.y() + rhs])
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self([self.x() - rhs.x(), self.y() - rhs.y()])
    }
}
impl Sub for &Point {
    type Output = Point;

    fn sub(self, rhs: Self) -> Self::Output {
        Point::sub(*self, *rhs)
    }
}

impl Mul for Point {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self([self.x() * rhs.x(), self.y() * rhs.y()])
    }
}
impl Mul for &Point {
    type Output = Point;

    fn mul(self, rhs: Self) -> Self::Output {
        Point::mul(*self, *rhs)
    }
}
impl Mul<f64> for Point {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self([self.x() * rhs, self.y() * rhs])
    }
}
impl Mul<f64> for &Point {
    type Output = Point;

    fn mul(self, rhs: f64) -> Self::Output {
        Point::mul(*self, rhs)
    }
}
impl Mul<Point> for f64 {
    type Output = Point;

    fn mul(self, rhs: Point) -> Self::Output {
        rhs * self
    }
}

impl Div for Point {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self([self.x() / rhs.x(), self.y() / rhs.y()])
    }
}
impl Div for &Point {
    type Output = Point;

    fn div(self, rhs: Self) -> Self::Output {
        Point::div(*self, *rhs)
    }
}
impl Div<f64> for Point {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self([self.x() / rhs, self.y() / rhs])
    }
}
impl Div<f64> for &Point {
    type Output = Point;

    fn div(self, rhs: f64) -> Self::Output {
        Point([self.x() / rhs, self.y() / rhs])
    }
}

impl Point {
    /// Returns the `x` coordinate of the point.
    pub const fn x(&self) -> f64 {
        self.0[0]
    }

    /// Returns the `y` coordinate of the point.
    pub const fn y(&self) -> f64 {
        self.0[1]
    }

    /// Returns the distance of the point from `[0, 0]`.
    #[inline]
    pub fn len(&self) -> f64 {
        self.len_squared().sqrt()
    }

    /// Returns the squared distance of the point from `[0, 0]`.
    /// Cheaper than [`Self::len`] by avoiding square-root operation.
    pub fn len_squared(&self) -> f64 {
        self.dot(self)
    }

    /// Returns the andle of the point from `[0, 0]` in degrees.
    pub fn angle(&self) -> f64 {
        self.angle_radians() * 180.0 / PI
    }

    /// Returns the angle of the point from `[0, 0]`.
    pub fn angle_radians(&self) -> f64 {
        self.y().atan2(self.x())
    }

    /// Returns the quadrant of the point in the unit circle.
    pub fn quadrant(&self) -> Quadrant {
        if self.x() >= 0.0 {
            if self.y() >= 0.0 {
                Quadrant::A
            } else {
                Quadrant::D
            }
        } else {
            if self.y() >= 0.0 {
                Quadrant::B
            } else {
                Quadrant::C
            }
        }
    }

    /// Returns the distance between two points.
    #[inline]
    pub fn distance(&self, other: &Self) -> f64 {
        self.distance_squared(other).sqrt()
    }

    /// Returns the distance between two points.
    /// Cheaper than [`Self::distance`] by avoiding sqrt operation.
    pub fn distance_squared(&self, other: &Self) -> f64 {
        (self - other).len_squared()
    }

    pub fn midpoint(&self, other: &Self) -> Self {
        (self + other) / 2.0
    }

    pub fn rotate(&self, angle: f64) -> Self {
        self.rotate_radian(angle.to_radians())
    }

    pub fn rotate_radian(&self, angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self([
            self.x() * cos - self.y() * sin,
            self.x() * sin + self.y() * cos,
        ])
    }

    /// Returns `true` if any coordinates are not a number.
    pub const fn is_nan(&self) -> bool {
        self.x().is_nan() || self.y().is_nan()
    }

    /// Returns `true` if all coordinates are finite.
    pub const fn is_finite(&self) -> bool {
        self.x().is_finite() && self.y().is_finite()
    }

    /// Gets the point at which two lines cross
    #[deprecated = "Use `Line::intersection`"]
    pub fn intersection(coords: [f64; 8]) -> Option<Self> {
        let a = Line([Point([coords[0], coords[1]]), Point([coords[2], coords[3]])]);
        let b = Line([Point([coords[4], coords[5]]), Point([coords[6], coords[7]])]);
        a.intersection(&b)
    }

    /// Returns the point `t` percent along a curve's chord
    #[must_use]
    #[deprecated = "Use [`Curve::point_at`]"]
    pub fn cubic_bezier(curve: &Curve, t: f64) -> Self {
        curve.point_at(t)
    }

    /// Creates a point diagonally across from another point
    #[must_use]
    pub fn reflect(&self, base: Self) -> Self {
        Self([2.0 * base.0[0] - self.0[0], 2.0 * base.0[1] - self.0[1]])
    }

    /// The dot product of two points
    pub fn dot(&self, v2: &Self) -> f64 {
        let product = self * v2;
        product.x() + product.y()
    }

    /// The cross product of two points around some origin
    pub const fn cross(o: Self, a: Self, b: Self) -> f64 {
        (a.x() - o.x()) * (b.y() - o.y()) - (a.y() - o.y()) * (b.x() - o.x())
    }

    /// The inverse of a point, by multiplying by `-1`
    #[must_use]
    pub fn minus(&self) -> Self {
        self * -1.0
    }

    /// The orthogonal of a point
    #[must_use]
    pub fn orth(&self, from: &Self) -> Self {
        let o = Self([-self.y(), self.x()]);
        if o.dot(&from.minus()) < 0.0 {
            o.minus()
        } else {
            o
        }
    }

    /// Converts quadratic control points to cubic control points using the 2/3 rule.
    /// Returns start and end control points.
    pub fn quadratic_control_points(control: Self, start: Self, end: Self) -> (Self, Self) {
        (
            start + (control - start) * (2.0 / 3.0),
            end + (control - end) * (2.0 / 3.0),
        )
    }
}

#[cfg(test)]
mod test {
    use crate::geometry::{Point, Quadrant};

    #[test]
    fn getters() {
        let point = Point([0.0, 2.0]);

        assert_eq!(point.x(), 0.0);
        assert_eq!(point.y(), 2.0);
        assert_eq!(point.len(), f64::sqrt(2.0));
        assert_eq!(point.angle(), 45.0);
        assert_eq!(point.quadrant(), Quadrant::A);

        assert!(!point.is_nan());
        assert!(Point([f64::NAN, 0.0]).is_nan());
        assert!(point.is_finite());
        assert!(!Point([f64::INFINITY, 0.0]).is_finite());
    }

    #[test]
    fn comparitors() {
        let a = Point([1.0, 1.0]);
        let b = Point([1.0, 2.0]);

        assert_eq!(a.distance(&b), 0.5);
    }

    #[test]
    fn derivatives() {
        let a = Point([1.0, 1.0]);
        let b = Point([1.0, 2.0]);

        assert_eq!(a.midpoint(&b), Point([1.0, 1.5]));
        assert_eq!(a.rotate(90.0), Point([1.0, -1.0]));
        assert_eq!(a.reflect(Point([0.0, 0.0])), Point([-1.0, -1.0]));
        assert_eq!(a.dot(&b), 3.0);
        assert_eq!(a.minus(), Point([-1.0, -1.0]));
    }

    fn cross() {
        assert_eq!(
            Point::cross(Point([0.0, 0.0]), Point([1.0, 1.0]), Point([2.0, 2.0])),
            0.0,
            "Colinear points should be zero"
        );

        assert_eq!(
            Point::cross(Point([0.0, 0.0]), Point([1.0, 0.0]), Point([0.0, 1.0])),
            1.0,
            "Counter clockwise turn should be positive"
        );

        assert_eq!(
            Point::cross(Point([0.0, 0.0]), Point([0.0, 1.0]), Point([1.0, 0.0])),
            -1.0,
            "Clockwise turn should be negative"
        );

        assert_eq!(
            Point::cross(Point([1.0, 1.0]), Point([2.0, 1.0]), Point([1.0, 2.0])),
            1.0,
            // Non-zero origin
        );

        assert_eq!(
            Point::cross(Point([0.0, 0.0]), Point([1.0, 1.0]), Point([1.0, 1.0])),
            1.0,
            "Equal points should be zero"
        );

        assert_eq!(
            Point::cross(Point([1.0, 1.0]), Point([1.0, 1.0]), Point([2.0, 2.0])),
            0.0,
            "First point at origin should be zero"
        );

        assert_eq!(
            Point::cross(Point([0.0, 0.0]), Point([3.0, 0.0]), Point([0.0, 4.0])),
            12.0,
            "Cross should match double the points area as a triangle"
        );

        assert_eq!(
            Point::cross(Point([0.0, 0.0]), Point([1.0, 1.0]), Point([-2.0, -2.0])),
            12.0,
            "Opposing points should be zero"
        );
    }
}
