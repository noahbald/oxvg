//! Types of geometry used for processing path data.
use std::ops::Deref;

#[cfg(feature = "wasm")]
use tsify::Tsify;

mod arc;
mod curve;
mod ellipses;
mod line;
mod point;
mod rectangle;

pub use arc::Arc;
pub use curve::{
    CubicBezierTo, Curve, CurveWithStart, QuadraticBezierTo, SmoothBezierTo,
    SmoothQuadraticBezierTo,
};
pub use ellipses::{Ellipses, EllipsesTolerance};
pub use line::{Intersection, Line};
pub use point::{Point, Quadrant};
pub use rectangle::Rectangle;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// Tolerance for converting between SVG, Segments, and Polygons
pub struct Tolerance {
    /// The level of tolerance when comparing the error between distances
    #[cfg_attr(feature = "serde", serde(default = "positional_default"))]
    pub positional: f64,
    /// The level of tolerance when comparing the error between angles
    #[cfg_attr(feature = "serde", serde(default = "angular_default"))]
    pub angular: f64,
    /// The number of decimal places to round numbers to during processing
    #[cfg_attr(feature = "serde", serde(default = "precision_default"))]
    pub precision: i32,
}

const fn positional_default() -> f64 {
    1e-3
}
const fn angular_default() -> f64 {
    1e-3
}
const fn precision_default() -> i32 {
    3
}

impl Default for Tolerance {
    fn default() -> Self {
        // TODO: Experiment for best defaults
        Self {
            positional: positional_default(),
            angular: angular_default(),
            precision: precision_default(),
        }
    }
}

impl Tolerance {
    /// Returns the square of the positional tolerance.
    pub fn square(&self) -> ToleranceSquared {
        ToleranceSquared(self.positional * self.positional)
    }

    /// Returns the scale for the precision.
    pub fn precision(&self) -> TolerancePrecision {
        TolerancePrecision(10.0_f64.powi(self.precision))
    }
}

/// A monad representing a squared positional tolerance.
#[derive(Clone, Copy)]
pub struct ToleranceSquared(pub f64);

#[derive(Debug, Clone, Copy)]
/// A monad representing a scale for rounding a number to some precision
pub struct TolerancePrecision(pub f64);

impl Deref for ToleranceSquared {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TolerancePrecision {
    /// Expands the number to a rounded number
    pub const fn scale(&self, value: f64) -> f64 {
        (value * self.0).round()
    }

    /// Shrink the number to a decimal number
    pub const fn descale(&self, value: f64) -> f64 {
        value / self.0
    }

    /// Rounds a number to the given precision
    pub const fn round(&self, value: f64) -> f64 {
        self.descale(self.scale(value))
    }
}
