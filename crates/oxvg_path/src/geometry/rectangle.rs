use std::ops::{Deref, DerefMut};

use geo::Coord;

use crate::geometry::Point;

/// A bounded 2D quadrilateral whose area is defined by minimum and maximum [`Point`].
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Rectangle(pub geo_types::Rect<f64>);

impl Deref for Rectangle {
    type Target = geo_types::Rect<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Rectangle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Rectangle {
    /// Returns a rectangle covering the minimum and maximum of the given terminals
    pub fn new(a: Point, b: Point) -> Self {
        Self(geo_types::Rect::new(a.0, b.0))
    }

    pub fn from_points<'a>(iter: impl Iterator<Item = &'a Point>) -> Self {
        Self::from_coords(iter.map(Deref::deref))
    }

    pub fn from_coords<'a>(iter: impl Iterator<Item = &'a geo_types::Coord<f64>>) -> Self {
        let mut min_x = f64::NAN;
        let mut max_x = f64::NAN;
        let mut min_y = f64::NAN;
        let mut max_y = f64::NAN;

        for coord in iter {
            min_x = min_x.min(coord.x);
            max_x = max_x.max(coord.x);
            min_y = min_y.min(coord.y);
            max_y = max_y.max(coord.y);
        }
        Self(geo_types::Rect::new(
            geo_types::Coord { x: min_x, y: min_y },
            geo_types::Coord { x: max_x, y: max_y },
        ))
    }

    /// Returns the rectangle that fits within the two rectangles
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let min_x = self.min().x.max(other.min().x);
        let min_y = self.min().y.max(other.min().y);
        let max_x = self.max().x.min(other.max().x);
        let max_y = self.max().y.min(other.max().y);

        if min_x <= max_x && min_y <= max_y {
            Some(Self::new(
                Point::new(min_x, min_y),
                Point::new(max_x, max_y),
            ))
        } else {
            None
        }
    }

    /// Returns whether the two rectangle overlap each other
    pub fn intersects(&self, other: &Self) -> bool {
        self.min().x <= other.max().x
            && self.max().x >= other.min().x
            && self.min().y <= other.max().y
            && other.max().y >= other.min().y
    }

    /// Returns whether the rectangle contains the given point
    pub fn contains(&self, point: Point) -> bool {
        self.min().x <= point.x
            && point.x <= self.max().x
            && self.min().y <= point.y
            && point.y <= self.max().y
    }

    /// Returns a point clamped within the bounds of the rectangle
    pub fn clamp(&self, point: Point) -> Point {
        Point::new(
            point.x.clamp(self.min().x, self.max().x),
            point.y.clamp(self.min().y, self.max().y),
        )
    }
}
