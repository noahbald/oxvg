//! Types of geometry used for processing path data.
#[cfg(feature = "wasm")]
use tsify::Tsify;

mod arc;
mod circle;
mod curve;
mod line;
mod point;
mod polygon;

pub use arc::Arc;
pub use circle::Circle;
pub use curve::Curve;
pub use line::Line;
pub use point::{Point, Quadrant};
pub use polygon::Polygon;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
/// When running calculations against curves and arcs, the level of error tolerated
pub struct ErrorOptions {
    /// When calculating tolerance, controls the bound compared to error
    pub threshold: f64,
    /// When calculating tolerance, controls the bound compared to the radius
    pub tolerance: f64,
}

/// When running calculations against curves and arcs, the level of error tolerated
#[deprecated = "Use `[CurveError]`"]
pub type MakeArcs = ErrorOptions;

impl Default for ErrorOptions {
    fn default() -> Self {
        Self {
            threshold: 2.5,
            tolerance: 0.5,
        }
    }
}
