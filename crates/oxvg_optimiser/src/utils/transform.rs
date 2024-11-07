use std::fmt::{Debug, Write};

use itertools::Itertools;
use lightningcss::{
    properties::transform::{self},
    values::{
        number::CSSNumber,
        percentage::{DimensionPercentage, NumberOrPercentage},
    },
};

#[derive(Clone, Debug)]
pub enum Transform {
    Matrix(Matrix),
    Translate((CSSNumber, Option<CSSNumber>)),
    Scale((CSSNumber, Option<CSSNumber>)),
    Rotate((CSSNumber, Option<(CSSNumber, CSSNumber)>)),
    SkewX(CSSNumber),
    SkewY(CSSNumber),
}

#[derive(Clone, Debug)]
pub struct Matrix(pub transform::Matrix<CSSNumber>);

impl TryFrom<&transform::Transform> for Transform {
    type Error = ();

    fn try_from(value: &transform::Transform) -> Result<Self, Self::Error> {
        Ok(match value {
            transform::Transform::Matrix(matrix) => Self::Matrix(Matrix(matrix.clone())),
            transform::Transform::Translate(
                DimensionPercentage::Dimension(x),
                DimensionPercentage::Dimension(y),
            ) => {
                let Some(x) = x.to_px() else { return Err(()) };
                let Some(y) = y.to_px() else { return Err(()) };
                Self::Translate((x, Some(y)))
            }
            transform::Transform::TranslateX(DimensionPercentage::Dimension(x)) => {
                let Some(x) = x.to_px() else { return Err(()) };
                Self::Translate((x, None))
            }
            transform::Transform::Scale(x, y) => {
                let NumberOrPercentage::Number(x) = x else {
                    return Err(());
                };
                let NumberOrPercentage::Number(y) = y else {
                    return Err(());
                };
                Self::Scale((*x, Some(*y)))
            }
            transform::Transform::ScaleX(x) => {
                let NumberOrPercentage::Number(x) = x else {
                    return Err(());
                };
                Self::Scale((*x, None))
            }
            transform::Transform::Rotate(angle) => Self::Rotate((angle.to_degrees(), None)),
            transform::Transform::SkewX(x) => Self::SkewX(x.to_degrees()),
            transform::Transform::SkewY(y) => Self::SkewY(y.to_degrees()),
            _ => return Err(()),
        })
    }
}

#[derive(Debug)]
pub struct Precision {
    pub float: i32,
    pub deg: i32,
    pub transform: i32,
}

impl Transform {
    pub fn round(&mut self, precision: &Precision) {
        match self {
            Transform::Translate((x, y)) => {
                let precision = precision.float;
                Self::round_arg(precision, x);
                if let Some(y) = y {
                    Self::round_arg(precision, y);
                }
            }
            Transform::Rotate((d, coords)) => {
                Self::round_arg(precision.deg, d);
                if let Some((x, y)) = coords {
                    let precision = precision.float;
                    Self::round_arg(precision, x);
                    Self::round_arg(precision, y);
                }
            }
            Transform::SkewX(a) | Transform::SkewY(a) => Self::round_arg(precision.deg, a),
            Transform::Scale((x, y)) => {
                let precision = precision.transform;
                Self::round_arg(precision, x);
                if let Some(y) = y {
                    Self::round_arg(precision, y);
                }
            }
            Transform::Matrix(Matrix(m)) => {
                let p = precision.transform;
                Self::round_arg(p, &mut m.a);
                Self::round_arg(p, &mut m.b);
                Self::round_arg(p, &mut m.c);
                Self::round_arg(p, &mut m.d);
                let p = precision.float;
                Self::round_arg(p, &mut m.e);
                Self::round_arg(p, &mut m.f);
            }
        };
    }

    fn round_vec(transforms: &mut [Transform], precision: &Precision) {
        transforms
            .iter_mut()
            .for_each(|transform| transform.round(precision));
    }

    fn round_arg(precision: i32, data: &mut f32) {
        *data = if (1..20).contains(&precision) {
            smart_round(precision, *data)
        } else {
            data.round()
        }
    }

    fn optimize(rounded: &[Transform], raw: &[Transform]) -> Vec<Transform> {
        assert_eq!(rounded.len(), raw.len());

        let mut optimized = vec![];
        let mut skip = false;
        for i in 0..rounded.len() {
            if skip {
                skip = false;
                continue;
            }
            let item = &rounded[i];
            if item.is_identity() {
                continue;
            }
            match item {
                Self::Rotate((n, _)) if n.abs() == 180.0 => {
                    if let Some(Self::Scale((x, y))) = rounded.get(i + 1) {
                        optimized.push(Self::Scale((-x, y.map(|y| -y))));
                        skip = true;
                    } else {
                        optimized.push(Self::Scale((-1.0, None)));
                    };
                    continue;
                }
                Self::Rotate((n, Some((x, y)))) if *x == 0.0 && *y == 0.0 => {
                    optimized.push(Self::Rotate((*n, None)));
                }
                Self::Rotate((n, coords)) => {
                    optimized.push(Self::Rotate((*n, *coords)));
                }
                Self::Translate(_) => {
                    if let Some(Self::Rotate((n, Some((x, y))))) = rounded.get(i + 1) {
                        if *n != 180.0 && *n != -180.0 && *n != 0.0 && *x == 0.0 && *y == 0.0 {
                            log::debug!("merging translate and rotate");
                            let data = raw[i].data();
                            let next_data = raw[i + 1].data();
                            optimized.push(Self::merge_translate_and_rotate(
                                data[0],
                                data[1],
                                next_data[0],
                            ));
                            skip = true;
                            continue;
                        }
                    }
                    optimized.push(item.clone());
                }
                Self::Matrix(_) => unreachable!(),
                _ => optimized.push(item.clone()),
            };
        }
        let optimized = optimized
            .into_iter()
            .map(|t| match t {
                Self::Scale((x, Some(y))) if x == y => Self::Scale((x, None)),
                t => t,
            })
            .collect::<Vec<_>>();

        if optimized.is_empty() {
            vec![Transform::Scale((1.0, None))]
        } else {
            optimized
        }
    }

    fn is_identity(&self) -> bool {
        match self {
            Self::Rotate((n, _)) | Self::SkewX(n) | Self::SkewY(n) => *n == 0.0,
            Self::Scale((x, y)) | Self::Translate((x, y)) => *x == 1.0 && *y == Some(1.0),
            Self::Matrix(_) => false,
        }
    }

    fn merge_translate_and_rotate(tx: f32, ty: f32, a: f32) -> Self {
        let rad = a.to_radians();
        let d = 1.0 - rad.cos();
        let e = rad.sin();
        let cy = (d * ty + e * tx) / (d * d + e * e);
        let cx = (tx - e * cy) / d;
        Self::Rotate((a, Some((cx, cy))))
    }

    fn data(&self) -> Vec<f32> {
        match self {
            Self::Scale((x, None)) | Self::Translate((x, None)) | Self::SkewX(x) => vec![*x],
            Self::SkewY(y) => vec![*y],
            Self::Translate((x, Some(y))) | Self::Scale((x, Some(y))) => vec![*x, *y],
            Self::Matrix(Matrix(m)) => vec![m.a, m.b, m.c, m.d, m.e, m.f],
            Self::Rotate((deg, Some((x, y)))) => vec![*deg, *x, *y],
            Self::Rotate((deg, None)) => vec![*deg],
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Scale(_) => "scale",
            Self::Matrix(_) => "matrix",
            Self::Translate(_) => "translate",
            Self::Rotate(_) => "rotate",
            Self::SkewX(_) => "skewX",
            Self::SkewY(_) => "skewY",
        }
    }
}

impl Matrix {
    pub fn to_transform(&self, precision: &Precision) -> Vec<Transform> {
        let decomposed = self.get_compositions();

        let mut shortest = vec![Transform::Matrix(self.clone())];
        Transform::round_vec(&mut shortest, precision);
        let starting_string = shortest[0].to_string();
        let mut shortest_len = starting_string.len();
        log::debug!("converting matrix to transform: {starting_string} ({shortest_len})");
        for decomposition in decomposed {
            let mut rounded_transforms = decomposition.clone();
            Transform::round_vec(&mut rounded_transforms, precision);

            let mut optimized = Transform::optimize(&rounded_transforms, &decomposition);
            Transform::round_vec(&mut optimized, precision);
            let optimized_string = optimized.iter().map(|item| format!("{item}")).join("");
            log::debug!("optimized: {optimized_string} ({})", optimized_string.len());
            if optimized_string.len() <= shortest_len {
                shortest = optimized;
                shortest_len = optimized_string.len();
            }
        }

        log::debug!(r#"converted to transform: {:?}"#, shortest);
        shortest
    }

    fn get_compositions(&self) -> Vec<Vec<Transform>> {
        let mut decompositions = vec![];

        if let Some(qrcd) = self.qrcd() {
            log::debug!(r#"decomposed qrcd: "{:?}""#, qrcd);
            decompositions.push(qrcd);
        }
        if let Some(qrab) = self.qrab() {
            log::debug!(r#"decomposed qrab: "{:?}""#, qrab);
            decompositions.push(qrab);
        }
        decompositions
    }

    fn qrab(&self) -> Option<Vec<Transform>> {
        let transform::Matrix { a, b, c, d, e, f } = self.0;
        let delta = a * d - b * c;
        if delta == 0.0 {
            return None;
        }

        let radius = f32::hypot(a, b);
        if radius == 0.0 {
            return None;
        }

        let mut decomposition = vec![];
        let cos = a / radius;

        if e != 0.0 || f != 0.0 {
            decomposition.push(Transform::Translate((e, Some(f))));
        }

        if cos != 1.0 {
            let mut rad = cos.acos();
            if b < 0.0 {
                rad *= -1.0;
            }
            decomposition.push(Transform::Rotate((rad.to_degrees(), Some((0.0, 0.0)))));
        }

        let sx = radius;
        let sy = delta / sx;
        if sx != 1.0 || sy != 1.0 {
            decomposition.push(Transform::Scale((sx, Some(sy))));
        }

        let ac_bd = a * c + b * d;
        if ac_bd != 0.0 {
            decomposition.push(Transform::SkewX(
                (ac_bd / (a * a + b * b)).atan().to_degrees(),
            ));
        }

        Some(decomposition)
    }

    fn qrcd(&self) -> Option<Vec<Transform>> {
        let transform::Matrix { a, b, c, d, e, f } = self.0;

        let delta = a * d - b * c;
        if delta == 0.0 {
            return None;
        }
        let s = f32::hypot(c, d);
        if s == 0.0 {
            return None;
        }

        let mut decomposition = vec![];

        if e != 0.0 || f != 0.0 {
            decomposition.push(Transform::Translate((e, Some(f))));
        }

        let rad =
            std::f32::consts::PI / 2.0 - (if d < 0.0 { -1.0 } else { 1.0 }) * f32::acos(-c / s);
        decomposition.push(Transform::Rotate((rad.to_degrees(), Some((0.0, 0.0)))));

        let sx = delta / s;
        let sy = s;
        if sx != 1.0 || sy != 1.0 {
            decomposition.push(Transform::Scale((sx, Some(sy))));
        }

        let ac_bd = a * c + b * d;
        if ac_bd != 0.0 {
            decomposition.push(Transform::SkewY(
                f32::atan(ac_bd / (c * c + d * d)).to_degrees(),
            ));
        }

        Some(decomposition)
    }
}

impl std::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())?;
        f.write_char('(')?;
        self.data()
            .iter()
            .enumerate()
            .try_for_each(|(i, n)| -> std::fmt::Result {
                if i > 0 {
                    f.write_char(' ')?;
                }
                if n > &0.0 && n < &1.0 {
                    let mut n_string = n.to_string();
                    n_string.remove(0);
                    f.write_str(&n_string)
                } else if n < &0.0 && n > &-1.0 {
                    let mut n_string = n.to_string();
                    n_string.remove(1);
                    f.write_str(&n_string)
                } else {
                    std::fmt::Display::fmt(n, f)
                }
            })?;
        f.write_char(')')
    }
}

impl From<Transform> for String {
    fn from(value: Transform) -> Self {
        let mut output = String::with_capacity(8); // skewX(n) -> 8 is shortest possible
        write!(output, "{value}").unwrap();
        output
    }
}

pub fn smart_round(precision: i32, data: f32) -> f32 {
    let tolerance = to_fixed(0.1_f32.powi(precision), precision);
    if to_fixed(data, precision) == data {
        data
    } else {
        let rounded = to_fixed(data, precision - 1);
        if to_fixed((rounded - data).abs(), precision + 1) >= tolerance {
            to_fixed(data, precision)
        } else {
            rounded
        }
    }
}

fn to_fixed(data: f32, precision: i32) -> f32 {
    let pow = 10.0_f32.powi(precision);
    f32::round(data * pow) / pow
}
