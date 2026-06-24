use std::f64::consts::PI;

use crate::{geometry::Point, optimize::Tolerance, paths::segment::TolerancePrecision};

/// Rounds a number to a specified number of decimal points
pub fn to_fixed(data: f64, precision: i32) -> f64 {
    let pow = 10.0_f64.powi(precision);
    (data * pow).round() / pow
}

/// Calculate the hypotenuse of two numbers
pub(crate) fn hypot_squared(v1: f64, v2: f64) -> f64 {
    (v1 * v1) + (v2 * v2)
}

/// Calculates the saggita of an arc, clamped at 180 degrees.
///
/// A saggita is the distance from the midpoint of the arc to itself
pub(crate) fn saggita(arc_by: &[f64; 7], error: f64) -> Option<f64> {
    let [rx, ry, ..] = arc_by;
    if (rx - ry).abs() > error {
        return None;
    }
    let chord_squared = hypot_squared(arc_by[5], arc_by[6]);
    if chord_squared > (rx * 2.0).powi(2) {
        return None;
    }
    let saggita = rx - f64::sqrt((rx * rx) - 0.25 * chord_squared);
    if (arc_by[3] - 1.0).abs() < f64::EPSILON {
        Some((2.0 * rx - saggita).min(*rx))
    } else {
        Some(saggita)
    }
}

/// A greatest common denominator algorithm that exits early when the denominator is within some
/// tolerance truncated by some precision
#[allow(clippy::many_single_char_names)]
pub fn euclid_gcd_lossy(
    a: f64,
    b: f64,
    tolerance: &Tolerance,
    precision: &TolerancePrecision,
) -> f64 {
    #[allow(clippy::cast_sign_loss)]
    let u = precision.scale(a).abs() as u32;
    #[allow(clippy::cast_sign_loss)]
    let v = precision.scale(b).abs() as u32;
    let (mut u, mut v) = if u > v { (u, v) } else { (v, u) };

    #[allow(clippy::manual_swap)]
    while v != 0 {
        // mem::swap(&mut a, &mut b);
        let temp = u;
        u = v;
        v = temp;

        v %= u;

        // If dividing out the current u would bring both components within
        // positional tolerance of a rounded value, we can stop early.
        let g = precision.descale(u as f64);
        let ra = a / g;
        let rb = b / g;
        if (ra - ra.round()).abs() < tolerance.positional
            && (rb - rb.round()).abs() < tolerance.positional
        {
            return g;
        }
    }

    precision.descale(u as f64)
}

/// Returns the radius factor as defined by the SVG spec (F6.6.2)
pub fn radius_factor(rx: f64, ry: f64, xr: f64, start: Point, end: Point) -> f64 {
    // Step 1: Ensure radii are non-zero
    if rx == 0.0 || ry == 0.0 {
        return 0.0;
    }
    // Step 2: Ensure radii are positive
    let rx = rx.abs();
    let ry = ry.abs();

    // Step 3: Ensure radii are large enough
    let xr = xr % (2.0 * PI);

    let (sin_phi, cos_phi) = xr.sin_cos();
    let x_mid = (start.x - end.x) * 0.5;
    let y_mid = (start.y - end.y) * 0.5;
    let x_prime = cos_phi * x_mid + sin_phi * y_mid;
    let y_prime = -sin_phi * x_mid + cos_phi * y_mid;

    x_prime.powi(2) / rx.powi(2) + y_prime.powi(2) / ry.powi(2)
}
