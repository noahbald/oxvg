pub fn to_fixed(data: f64, precision: i32) -> f64 {
    let pow = 10.0_f64.powi(precision);
    (data * pow).round() / pow
}

pub(crate) fn hypot(v1: f64, v2: f64) -> f64 {
    f64::sqrt((v1 * v1) + (v2 * v2))
}

pub fn saggita(args: &[f64; 7], error: f64) -> Option<f64> {
    if (args[3] - 1.0).abs() < f64::EPSILON {
        return None;
    }
    let [rx, ry, ..] = args;
    if f64::abs(rx - ry) > error {
        return None;
    }
    let chord = hypot(args[5], args[6]);
    if chord > rx * 2.0 {
        return None;
    }
    Some(rx - f64::sqrt((rx * rx) - 0.25 * (chord * chord)))
}
