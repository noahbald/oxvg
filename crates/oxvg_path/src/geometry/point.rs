// Implementations based off `glam::f64::DVec2`.
use std::{
    f64::consts::PI,
    ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::optimize::Tolerance;

#[derive(Default, Clone, Copy, PartialEq)]
/// A point is an `[x, y]` coordinate. Points are the atomic unit of geometry.
pub struct Point(pub geo_types::Coord<f64>);

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

impl std::fmt::Debug for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.0))
    }
}

impl Deref for Point {
    type Target = geo_types::Coord<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Point {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0 + rhs.0;
    }
}
impl Div<f64> for Point {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self(self.0 / rhs)
    }
}
impl DivAssign<f64> for Point {
    fn div_assign(&mut self, rhs: f64) {
        self.0 = self.0 / rhs
    }
}
impl From<(f64, f64)> for Point {
    fn from(value: (f64, f64)) -> Self {
        Self(value.into())
    }
}
impl From<geo_types::Coord<f64>> for Point {
    fn from(value: geo_types::Coord<f64>) -> Self {
        Self(value.into())
    }
}
impl From<[f64; 2]> for Point {
    fn from(value: [f64; 2]) -> Self {
        Self(value.into())
    }
}
impl From<Point> for [f64; 2] {
    fn from(value: Point) -> Self {
        [value.x, value.y]
    }
}
impl From<Point> for (f64, f64) {
    fn from(value: Point) -> Self {
        (value.x, value.y)
    }
}
impl From<Point> for geo_types::Point {
    fn from(value: Point) -> Self {
        geo_types::Point(value.0)
    }
}
impl From<Point> for geo_types::Coord {
    fn from(value: Point) -> Self {
        value.0
    }
}
impl Mul<f64> for Point {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}
impl MulAssign<f64> for Point {
    fn mul_assign(&mut self, rhs: f64) {
        self.0 = self.0 * rhs;
    }
}
impl Neg for Point {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
impl Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0 - rhs.0;
    }
}

impl Point {
    /// A zero vector point at (0, 0)
    pub const ZERO: Self = Self::splat(0.0);
    /// A unit vector point at (1, 1)
    pub const UNIT: Self = Self::splat(1.0);
    /// A negative unit vector point at (-1, -1)
    pub const NEG_UNIT: Self = Self::splat(-1.0);
    /// A 1 length vector point in the positive x direction (1, 0)
    pub const X: Self = Self::new(1.0, 0.0);
    /// A 1 length vector point in the negative x direction (-1, 0)
    pub const NEG_X: Self = Self::new(-1.0, 0.0);
    /// A 1 length vector point in the positive y direction (0, 1)
    pub const Y: Self = Self::new(0.0, 1.0);
    /// A 1 length vector point in the negative y direction (0, 1)
    pub const NEG_Y: Self = Self::new(0.0, -1.0);
    /// A infinite length unit point at (inf, inf)
    pub const INFINITY: Self = Self::splat(f64::INFINITY);
    /// A negative infinite length unit point at (-inf, -inf)
    pub const NEG_INFINITY: Self = Self::splat(f64::NEG_INFINITY);
    /// A point at (NAN, NAN)
    pub const NAN: Self = Self::splat(f64::NAN);

    /// Creates a new point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self(geo_types::Coord { x, y })
    }

    /// Creates a point with the same x/y values.
    pub const fn splat(n: f64) -> Self {
        Self::new(n, n)
    }

    /// Returns a point with each coordinate modified by a mapping function `f`.
    #[must_use]
    pub fn map<F>(self, mut f: F) -> Self
    where
        F: FnMut(f64) -> f64,
    {
        Self::new(f(self.x), f(self.y))
    }

    /// Creates a point from `self` with the given value of `x`.
    #[must_use]
    pub fn with_x(mut self, x: f64) -> Self {
        self.x = x;
        self
    }

    /// Creates a point from `self` with the given value of `y`.
    #[must_use]
    pub fn with_y(mut self, y: f64) -> Self {
        self.y = y;
        self
    }

    /// Returns the distance of the point from `[0, 0]`.
    #[inline]
    pub fn len(self) -> f64 {
        self.len_squared().sqrt()
    }

    /// Returns the squared distance of the point from `[0, 0]`.
    /// Cheaper than [`Self::len`] by avoiding square-root operation.
    pub fn len_squared(self) -> f64 {
        self.dot(self)
    }

    /// Returns the andle of the point from `[0, 0]` in degrees.
    pub fn angle(self) -> f64 {
        self.angle_radians() * 180.0 / PI
    }

    /// Returns the angle of the point from `[0, 0]`.
    pub fn angle_radians(self) -> f64 {
        self.y.atan2(self.x)
    }

    /// Returns the quadrant of the point in the unit circle.
    pub fn quadrant(self) -> Quadrant {
        if self.x >= 0.0 {
            if self.y >= 0.0 {
                Quadrant::A
            } else {
                Quadrant::D
            }
        } else {
            if self.y >= 0.0 {
                Quadrant::B
            } else {
                Quadrant::C
            }
        }
    }

    /// Returns the point farthest to the left
    pub fn leftmost(self, other: Self) -> Self {
        if self.x <= other.x {
            self
        } else {
            other
        }
    }

    /// Returns the point farthest to the right
    pub fn rightmost(self, other: Self) -> Self {
        if self.x > other.x {
            self
        } else {
            other
        }
    }

    /// Returns the distance between two points.
    #[inline]
    pub fn distance(self, other: Self) -> f64 {
        self.distance_squared(other).sqrt()
    }

    /// Returns the distance between two points.
    /// Cheaper than [`Self::distance`] by avoiding sqrt operation.
    pub fn distance_squared(self, other: Self) -> f64 {
        (self - other).len_squared()
    }

    /// Returns the point halfway between the two points
    #[must_use]
    pub fn midpoint(self, other: Self) -> Self {
        Self::new(self.x.midpoint(other.x), self.y.midpoint(other.y))
    }

    /// Returns the point rotated around the origin by some degrees
    #[must_use]
    pub fn rotate(self, angle: f64) -> Self {
        self.rotate_radian(angle.to_radians())
    }

    /// Returns the point rotated around the origin by some radians
    #[must_use]
    pub fn rotate_radian(self, angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }

    /// Returns `true` if any coordinates are not a number.
    pub fn is_nan(self) -> bool {
        self.x.is_nan() || self.y.is_nan()
    }

    /// Returns `true` if all coordinates are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Creates a point diagonally across from another point
    #[must_use]
    pub fn reflect(self, base: Self) -> Self {
        Self::new(2.0 * base.x - self.x, 2.0 * base.y - self.y)
    }

    /// The product of the point's x/y values.
    pub fn product(self) -> f64 {
        self.x * self.y
    }

    /// The dot product of two points
    pub fn dot(self, other: Self) -> f64 {
        geo_types::Point(self.0).dot(geo_types::Point(other.0))
    }

    /// Returns a point with an x/y coordinate as the dot product of two points.
    #[must_use]
    pub fn dot_into(self, other: Self) -> Self {
        Self::splat(self.dot(other))
    }

    /// Returns a point containing the minimum values of each component of `self` and `other`.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    /// Returns a point containing the maximum values of each component of `self` and `other`.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }

    /// The cross product of two points around some origin
    pub fn cross(self, point_b: Self, point_c: Self) -> f64 {
        geo_types::Point(self.0)
            .cross_prod(geo_types::Point(point_b.0), geo_types::Point(point_c.0))
    }

    /// Returns whether the three unordered points lie on a line; i.e. a parallel
    pub fn is_parallel(a: Point, b: Point, c: Point, tolerance: &Tolerance) -> bool {
        a.cross(b, c).abs() < tolerance.positional
    }

    /// Returns whether the three ordered points make up a line; i.e. a parallel
    pub fn is_continuous_parallel(
        start: Point,
        middle: Point,
        end: Point,
        tolerance: &Tolerance,
    ) -> bool {
        Self::is_parallel(start, middle, end, tolerance)
            && (start - middle).dot(middle - end) >= 0.0
    }

    /// The orthogonal of a point
    #[must_use]
    pub fn orth(&self, from: Self) -> Self {
        let o = Self::new(-self.y, self.x);
        if o.dot(-from) < 0.0 {
            -o
        } else {
            o
        }
    }

    /// Converts quadratic control points to cubic control points using the 2/3 rule.
    /// Returns start and end control points.
    #[deprecated = "move to `curve`"]
    pub fn quadratic_control_points(control: Self, start: Self, end: Self) -> (Self, Self) {
        (
            start + (control - start) * (2.0 / 3.0),
            end + (control - end) * (2.0 / 3.0),
        )
    }

    /// Returns the point `t` percentage between this point and the other, as a number
    /// between `0.0` and `1.0`.
    #[must_use]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        self + (other - self) * t
    }
}

#[cfg(test)]
mod test {
    use crate::geometry::{Point, Quadrant};

    #[test]
    fn getters() {
        let point = Point::new(1.0, 1.0);

        assert_eq!(point.x, 1.0);
        assert_eq!(point.y, 1.0);
        assert_eq!(point.len(), f64::sqrt(2.0));
        assert_eq!(point.angle(), 45.0);
        assert_eq!(point.quadrant(), Quadrant::A);

        assert!(!point.is_nan());
        assert!(Point::new(f64::NAN, 0.0).is_nan());
        assert!(point.is_finite());
        assert!(!Point::new(f64::INFINITY, 0.0).is_finite());
    }

    #[test]
    fn comparitors() {
        let a = Point::new(1.0, 1.0);
        let b = Point::new(1.0, 2.0);

        assert_eq!(a.distance(b), 1.0);
    }

    #[test]
    fn derivatives() {
        let a = Point::new(1.0, 1.0);
        let b = Point::new(1.0, 2.0);

        assert_eq!(a.midpoint(b), Point::new(1.0, 1.5));
        assert_eq!(a.rotate(90.0), Point::new(-0.999_999_999_999_999_9, 1.0));
        assert_eq!(a.reflect(Point::new(0.0, 0.0)), Point::new(-1.0, -1.0));
        assert_eq!(a.dot(b), 3.0);
        assert_eq!(-a, Point::new(-1.0, -1.0));
    }

    #[test]
    fn cross() {
        assert_eq!(
            Point::new(0.0, 0.0).cross(Point::new(1.0, 1.0), Point::new(2.0, 2.0)),
            0.0,
            "Colinear points should be zero"
        );

        assert_eq!(
            Point::new(0.0, 0.0).cross(Point::new(1.0, 0.0), Point::new(0.0, 1.0)),
            1.0,
            "Counter clockwise turn should be positive"
        );

        assert_eq!(
            Point::new(0.0, 0.0).cross(Point::new(0.0, 1.0), Point::new(1.0, 0.0)),
            -1.0,
            "Clockwise turn should be negative"
        );

        assert_eq!(
            Point::new(1.0, 1.0).cross(Point::new(2.0, 1.0), Point::new(1.0, 2.0)),
            1.0,
            // Non-zero origin
        );

        assert_eq!(
            Point::new(0.0, 0.0).cross(Point::new(1.0, 1.0), Point::new(1.0, 1.0)),
            0.0,
            "Equal points should be zero"
        );

        assert_eq!(
            Point::new(1.0, 1.0).cross(Point::new(1.0, 1.0), Point::new(2.0, 2.0)),
            0.0,
            "First point at origin should be zero"
        );

        assert_eq!(
            Point::new(0.0, 0.0).cross(Point::new(3.0, 0.0), Point::new(0.0, 4.0)),
            12.0,
            "Cross should match double the points area as a triangle"
        );
    }
}
