use std::f64::consts::PI;

use geo::Area;

use crate::geometry::Ellipses;

impl Area<f64> for Ellipses {
    fn signed_area(&self) -> f64 {
        PI * self.radii.product()
    }

    fn unsigned_area(&self) -> f64 {
        self.signed_area().abs()
    }
}
