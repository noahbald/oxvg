use std::{f64::consts::PI, ops::Deref};

use crate::{
    geometry::{Arc, Curve, Intersection, Line, Point},
    optimize::Tolerance,
    paths::segment::ToleranceSquared,
};

#[derive(Debug)]
pub struct Ellipses {
    pub center: Point,
    pub radii: Point,
    pub x_rotation: f64,
}

/// A monad representing a squared positional tolerance relative to an ellipse radius.
pub struct EllipsesTolerance(pub f64);

impl Deref for EllipsesTolerance {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ellipses {
    pub const fn new(center: Point, radii: Point, x_rotation: f64) -> Self {
        Self {
            center,
            radii,
            x_rotation,
        }
    }

    pub const fn arc(&self, start_angle: f64, sweep_angle: f64) -> Arc {
        Arc::new(
            self.center,
            self.radii,
            start_angle,
            sweep_angle,
            self.x_rotation,
        )
    }

    pub fn circumference(&self) -> f64 {
        let sum = self.radii.x + self.radii.y;
        let factor = (3.0 + sum) * (self.radii.x + 3.0 * self.radii.y);
        PI * (3.0 * sum - factor.sqrt())
    }

    pub fn is_circle(&self, tolerance: &Tolerance) -> bool {
        (self.radii.x - self.radii.y).abs() < tolerance.positional
    }

    pub fn point_at_angle(&self, angle_radians: f64) -> Point {
        let radii = self.radii;
        let start = Point::new(radii.x * angle_radians.cos(), radii.y * angle_radians.sin());

        self.center + start.rotate_radian(self.x_rotation)
    }

    pub fn ellipse_tolerance(&self, tolerance: &ToleranceSquared) -> EllipsesTolerance {
        EllipsesTolerance((**tolerance) * (1.0 + self.radii.x.max(self.radii.y).powi(2)))
    }

    pub fn angle_at_point(&self, at: Point, tolerance: &EllipsesTolerance) -> Option<f64> {
        let local = at - self.center;
        let unrotated = local.rotate_radian(-self.x_rotation);
        let angle = (unrotated.y / self.radii.y).atan2(unrotated.x / self.radii.x);

        if self.point_at_angle(angle).distance_squared(at) <= **tolerance {
            Some(angle)
        } else {
            None
        }
    }

    pub fn fit_curve(curve: &Curve, start: Point, tolerance: &Tolerance) -> Option<Ellipses> {
        // Find circumcircle
        let middle = curve.point_at_from(start, 0.5);
        let end = curve.end_point;
        let scale = (start.distance_squared(end) * 1.2).max(start.distance_squared(end).powi(-1));

        let m1 = start.midpoint(middle);
        let d1 = middle - start;
        let l1 = Line::new(
            m1 - Point::new(d1.y, -d1.x) * scale,
            m1 + Point::new(d1.y, -d1.x) * scale,
        );

        let m2 = middle.midpoint(end);
        let d2 = end - middle;
        let l2 = Line::new(
            m2 - Point::new(d2.y, -d2.x) * scale,
            m2 + Point::new(d2.y, -d2.x) * scale,
        );

        let Intersection::Intersection(center) = l1.intersection(&l2) else {
            return None;
        };

        // validate circle
        let radius = center.distance(start);
        if radius < 1e15
            && [0.25, 0.75].into_iter().all(|t| {
                (curve.point_at_from(start, t).distance(center) - radius).abs()
                    <= tolerance.positional * radius
            })
        {
            Some(Self::new(center, Point::splat(radius), 0.0))
        } else {
            None
        }
    }
}
