//! The transform attribute type
use itertools::Itertools as _;
use lightningcss::{
    properties::transform::{Matrix, Matrix3d, Transform, TransformList},
    values::{
        length::LengthPercentage,
        percentage::{DimensionPercentage, NumberOrPercentage},
    },
};

#[cfg(feature = "parse")]
use cssparser_lightningcss::Token;
#[cfg(feature = "parse")]
use oxvg_parse::{
    error::{ParseError, ParseErrorKind},
    Parse, Parser,
};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, PrinterOptions, ToValue};

use super::core::{Angle, Number};

#[derive(Debug, Clone, PartialEq)]
/// A transform applied to an element and it's children
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/coords.html#TransformAttribute)
/// [w3 | SVG 2](https://svgwg.org/svg2-draft/coords.html#TransformProperty)
/// [MDN | transform](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/transform)
pub enum SVGTransform {
    /// A transformation as the matrix of six values
    Matrix(Matrix<f32>),
    /// A positional transformation in an `x` and/or `y` direction
    Translate(f32, f32),
    /// A size transformation in an `x` and/or `y` direction
    Scale(f32, f32),
    /// A rotational transform by `a` degrees around an `x` and `y` origin
    Rotate(f32, f32, f32),
    /// A skew transform in the `x` direction
    SkewX(f32),
    /// A skew transform in the `y` direction
    SkewY(f32),
    /// A transform provided in a CSS format
    CssTransform(Transform),
}

#[derive(Debug, Clone, PartialEq)]
/// A list of transform definitions applied to an element and it's children.
///
/// [MDN | transform](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/transform)
pub struct SVGTransformList(pub Vec<SVGTransform>);

#[derive(Debug)]
/// Precision options use for rounding different types
pub struct Precision {
    /// The precision to use when rounding translations
    pub float: i32,
    /// The precision to use when rounding degrees
    pub deg: i32,
    /// The precision to use when rounding transforms
    pub transform: i32,
}

impl SVGTransform {
    /// Round the arguments of the transform to a given precision
    pub fn round(&mut self, precision: &Precision) {
        match self {
            SVGTransform::Translate(x, y) => {
                let precision = precision.float;
                Precision::round_arg(precision, x);
                Precision::round_arg(precision, y);
            }
            SVGTransform::Rotate(d, x, y) => {
                Precision::round_arg(precision.deg, d);
                let precision = precision.float;
                Precision::round_arg(precision, x);
                Precision::round_arg(precision, y);
            }
            SVGTransform::SkewX(a) | SVGTransform::SkewY(a) => {
                Precision::round_arg(precision.deg, a);
            }
            SVGTransform::Scale(x, y) => {
                let precision = precision.transform;
                Precision::round_arg(precision, x);
                Precision::round_arg(precision, y);
            }
            SVGTransform::Matrix(m) => {
                let p = precision.transform;
                Precision::round_arg(p, &mut m.a);
                Precision::round_arg(p, &mut m.b);
                Precision::round_arg(p, &mut m.c);
                Precision::round_arg(p, &mut m.d);
                let p = precision.float;
                Precision::round_arg(p, &mut m.e);
                Precision::round_arg(p, &mut m.f);
            }
            SVGTransform::CssTransform(_) => {}
        }
    }

    fn round_vec(transforms: &mut [Self], precision: &Precision) {
        transforms
            .iter_mut()
            .for_each(|transform| transform.round(precision));
    }

    #[cfg(feature = "serialize")]
    /// Converts a matrix to the shortest form of a transform
    pub fn matrix_to_transform(&self, precision: &Precision) -> Vec<Self> {
        let mut shortest = vec![self.clone()];
        let Self::Matrix(m) = self else {
            return shortest;
        };

        let decomposed = Self::get_compositions(m);

        Self::round_vec(&mut shortest, precision);
        let Ok(starting_string) = shortest[0].to_value_string(PrinterOptions::default()) else {
            return vec![self.clone()];
        };
        let mut shortest_len = starting_string.len();
        log::debug!("converting matrix to transform: {starting_string} ({shortest_len})");
        for decomposition in decomposed {
            let mut rounded_transforms = decomposition.clone();
            Self::round_vec(&mut rounded_transforms, precision);

            let mut optimized = SVGTransformList::optimize(&rounded_transforms, &decomposition);
            Self::round_vec(&mut optimized, precision);
            let optimized_string = optimized
                .iter()
                .map(|item| {
                    item.to_value_string(PrinterOptions::default())
                        .unwrap_or_default()
                })
                .join("");
            log::debug!("optimized: {optimized_string} ({})", optimized_string.len());
            if optimized_string.len() <= shortest_len {
                shortest = optimized;
                shortest_len = optimized_string.len();
            }
        }

        log::debug!("converted to transform: {:?}", shortest);
        shortest
    }

    fn get_compositions(matrix: &Matrix<f32>) -> Vec<Vec<SVGTransform>> {
        let mut decompositions = vec![];

        if let Some(qrcd) = Self::qrcd(matrix) {
            log::debug!(r#"decomposed qrcd: "{:?}""#, qrcd);
            decompositions.push(qrcd);
        }
        if let Some(qrab) = Self::qrab(matrix) {
            log::debug!(r#"decomposed qrab: "{:?}""#, qrab);
            decompositions.push(qrab);
        }
        decompositions
    }

    fn qrab(matrix: &Matrix<f32>) -> Option<Vec<SVGTransform>> {
        let Matrix { a, b, c, d, e, f } = matrix;
        let delta = a * d - b * c;
        if delta == 0.0 {
            return None;
        }

        let radius = f32::hypot(*a, *b);
        if radius == 0.0 {
            return None;
        }

        let mut decomposition = vec![];
        let cos = a / radius;

        if *e != 0.0 || *f != 0.0 {
            decomposition.push(SVGTransform::Translate(*e, *f));
        }

        if cos != 1.0 {
            let mut rad = cos.acos();
            if *b < 0.0 {
                rad *= -1.0;
            }
            decomposition.push(SVGTransform::Rotate(rad.to_degrees(), 0.0, 0.0));
        }

        let sx = radius;
        let sy = delta / sx;
        if sx != 1.0 || sy != 1.0 {
            decomposition.push(SVGTransform::Scale(sx, sy));
        }

        let ac_bd = a * c + b * d;
        if ac_bd != 0.0 {
            decomposition.push(SVGTransform::SkewX(
                (ac_bd / (a * a + b * b)).atan().to_degrees(),
            ));
        }

        Some(decomposition)
    }

    fn qrcd(matrix: &Matrix<f32>) -> Option<Vec<SVGTransform>> {
        let Matrix { a, b, c, d, e, f } = matrix;

        let delta = a * d - b * c;
        if delta == 0.0 {
            return None;
        }
        let s = f32::hypot(*c, *d);
        if s == 0.0 {
            return None;
        }

        let mut decomposition = vec![];

        if *e != 0.0 || *f != 0.0 {
            decomposition.push(SVGTransform::Translate(*e, *f));
        }

        let rad =
            std::f32::consts::PI / 2.0 - (if *d < 0.0 { -1.0 } else { 1.0 }) * f32::acos(-c / s);
        decomposition.push(SVGTransform::Rotate(rad.to_degrees(), 0.0, 0.0));

        let sx = delta / s;
        let sy = s;
        if sx != 1.0 || sy != 1.0 {
            decomposition.push(SVGTransform::Scale(sx, sy));
        }

        let ac_bd = a * c + b * d;
        if ac_bd != 0.0 {
            decomposition.push(SVGTransform::SkewY(
                f32::atan(ac_bd / (c * c + d * d)).to_degrees(),
            ));
        }

        Some(decomposition)
    }

    fn is_identity(&self) -> bool {
        match self {
            Self::Rotate(n, _, _) | Self::SkewX(n) | Self::SkewY(n) => *n == 0.0,
            Self::Scale(x, y) | Self::Translate(x, y) => *x == 1.0 && *y == 1.0,
            Self::Matrix(_) | Self::CssTransform(_) => false,
        }
    }

    fn merge_translate_and_rotate(translate: &Self, rotate: &Self) -> Self {
        let Self::Translate(tx, ty) = translate else {
            unreachable!();
        };
        let Self::Rotate(a, 0.0, 0.0) = rotate else {
            unreachable!();
        };

        let rad = a.to_radians();
        let d = 1.0 - rad.cos();
        let e = rad.sin();
        let cy = (d * ty + e * tx) / (d * d + e * e);
        let cx = (tx - e * cy) / d;
        Self::Rotate(*a, cx, cy)
    }
}

#[cfg(feature = "parse")]
impl<'input> Parse<'input> for SVGTransform {
    #[allow(clippy::many_single_char_names)]
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                // SVG transforms allow whitespace between the function name and arguments, unlike CSS functions.
                // So this may tokenize either as a function, or as an ident followed by a parenthesis block.
                let function = input
                    .try_parse(|input| input.expect_function().cloned())
                    .or_else(|_| {
                        let name = input.expect_ident_cloned()?;
                        input.skip_whitespace();
                        input.expect_parenthesis_block()?;
                        Ok(name)
                    })
                    .map_err(ParseErrorKind::from_parser)?;

                let function_case_sensitive: &str = &function;
                input.parse_nested_block(|input| {
                    let location = input.current_source_location();
                    match function_case_sensitive {
                        "matrix" => {
                            let a = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let b = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let c = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let d = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let e = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let f = f32::parse(input)?;
                            Ok(SVGTransform::Matrix(Matrix { a, b, c, d, e, f }))
                        }
                        "translate" => {
                            let x = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            if let Ok(y) = input.try_parse(f32::parse) {
                                Ok(SVGTransform::Translate(x, y))
                            } else {
                                Ok(SVGTransform::Translate(x, 0.0))
                            }
                        }
                        "scale" => {
                            let x = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            if let Ok(y) = input.try_parse(f32::parse) {
                                Ok(SVGTransform::Scale(x, y))
                            } else {
                                Ok(SVGTransform::Scale(x, x))
                            }
                        }
                        "rotate" => {
                            let angle = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            if let Ok(x) = input.try_parse(f32::parse) {
                                skip_comma_and_whitespace(input);
                                let y = f32::parse(input)?;
                                Ok(SVGTransform::Rotate(angle, x, y))
                            } else {
                                Ok(SVGTransform::Rotate(angle, 0.0, 0.0))
                            }
                        }
                        "skewX" => {
                            let angle = f32::parse(input)?;
                            Ok(SVGTransform::SkewX(angle))
                        }
                        "skewY" => {
                            let angle = f32::parse(input)?;
                            Ok(SVGTransform::SkewY(angle))
                        }
                        _ => {
                            Err(location.new_unexpected_token_error(Token::Ident(function.clone())))
                        }
                    }
                })
            })
            .or_else(|_| Ok(SVGTransform::CssTransform(Transform::parse(input)?)))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for SVGTransform {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            SVGTransform::Matrix(Matrix { a, b, c, d, e, f }) => {
                dest.write_str("matrix(")?;
                a.write_value(dest)?;
                dest.write_char(' ')?;
                b.write_value(dest)?;
                dest.write_char(' ')?;
                c.write_value(dest)?;
                dest.write_char(' ')?;
                d.write_value(dest)?;
                dest.write_char(' ')?;
                e.write_value(dest)?;
                dest.write_char(' ')?;
                f.write_value(dest)?;
                dest.write_char(')')
            }
            SVGTransform::Translate(x, y) => {
                dest.write_str("translate(")?;
                x.write_value(dest)?;
                if *y != 0.0 {
                    dest.write_char(' ')?;
                    y.write_value(dest)?;
                }
                dest.write_char(')')
            }
            SVGTransform::Scale(x, y) => {
                dest.write_str("scale(")?;
                x.write_value(dest)?;
                if x != y {
                    dest.write_char(' ')?;
                    y.write_value(dest)?;
                }
                dest.write_char(')')
            }
            SVGTransform::Rotate(angle, x, y) => {
                dest.write_str("rotate(")?;
                angle.write_value(dest)?;
                if *x != 0.0 || *y != 0.0 {
                    dest.write_char(' ')?;
                    x.write_value(dest)?;
                    dest.write_char(' ')?;
                    y.write_value(dest)?;
                }
                dest.write_char(')')
            }
            SVGTransform::SkewX(angle) => {
                dest.write_str("skewX(")?;
                angle.write_value(dest)?;
                dest.write_char(')')
            }
            SVGTransform::SkewY(angle) => {
                dest.write_str("skewY(")?;
                angle.write_value(dest)?;
                dest.write_char(')')
            }
            SVGTransform::CssTransform(transform) => transform.write_value(dest),
        }
    }
}
impl From<SVGTransform> for Transform {
    fn from(val: SVGTransform) -> Self {
        match val {
            SVGTransform::Matrix(m) => Transform::Matrix(m),
            SVGTransform::Translate(x, y) => {
                Transform::Translate(LengthPercentage::px(x), LengthPercentage::px(y))
            }
            SVGTransform::Scale(x, y) => {
                Transform::Scale(NumberOrPercentage::Number(x), NumberOrPercentage::Number(y))
            }
            SVGTransform::Rotate(angle, x, y) => {
                if x == 0.0 && y == 0.0 {
                    return Transform::Rotate(Angle::Deg(angle));
                }
                let rad = angle.to_radians();
                let cos = rad.cos();
                let sin = rad.sin();
                Transform::Matrix(Matrix {
                    a: cos,
                    b: sin,
                    c: -sin,
                    d: cos,
                    e: (1.0 - cos) * x + sin * y,
                    f: (1.0 - cos) * y - sin * x,
                })
            }
            SVGTransform::SkewX(angle) => Transform::SkewX(Angle::Deg(angle)),
            SVGTransform::SkewY(angle) => Transform::SkewY(Angle::Deg(angle)),
            SVGTransform::CssTransform(transform) => transform,
        }
    }
}

impl TryFrom<&Transform> for SVGTransform {
    type Error = ();

    fn try_from(value: &Transform) -> Result<Self, Self::Error> {
        Ok(match value {
            Transform::Matrix(matrix) => Self::Matrix(matrix.clone()),
            Transform::Translate(
                DimensionPercentage::Dimension(x),
                DimensionPercentage::Dimension(y),
            ) => {
                let Some(x) = x.to_px() else { return Err(()) };
                let Some(y) = y.to_px() else { return Err(()) };
                Self::Translate(x, y)
            }
            Transform::TranslateX(DimensionPercentage::Dimension(x)) => {
                let Some(x) = x.to_px() else { return Err(()) };
                Self::Translate(x, 0.0)
            }
            Transform::Scale(x, y) => {
                let NumberOrPercentage::Number(x) = x else {
                    return Err(());
                };
                let NumberOrPercentage::Number(y) = y else {
                    return Err(());
                };
                Self::Scale(*x, *y)
            }
            Transform::ScaleX(x) => {
                let NumberOrPercentage::Number(x) = x else {
                    return Err(());
                };
                Self::Scale(*x, 0.0)
            }
            Transform::Rotate(angle) => Self::Rotate(angle.to_degrees(), 0.0, 0.0),
            Transform::SkewX(x) => Self::SkewX(x.to_degrees()),
            Transform::SkewY(y) => Self::SkewY(y.to_degrees()),
            t => match t.to_matrix() {
                Some(m) => match m.to_matrix2d() {
                    Some(m) => Self::Matrix(m),
                    None => return Err(()),
                },
                None => return Err(()),
            },
        })
    }
}
#[cfg(feature = "parse")]
fn skip_comma_and_whitespace(input: &mut Parser<'_, '_>) {
    input.skip_whitespace();
    let _ = input.try_parse(Parser::expect_comma);
    input.skip_whitespace();
}

impl SVGTransformList {
    fn optimize(rounded: &[SVGTransform], raw: &[SVGTransform]) -> Vec<SVGTransform> {
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
                SVGTransform::Rotate(n, 0.0, 0.0) if n.abs() == 180.0 => {
                    if let Some(SVGTransform::Scale(x, y)) = rounded.get(i + 1) {
                        optimized.push(SVGTransform::Scale(-x, -y));
                        skip = true;
                    } else {
                        optimized.push(SVGTransform::Scale(-1.0, -1.0));
                    }
                }
                SVGTransform::Rotate(..) => {
                    optimized.push(item.clone());
                }
                SVGTransform::Translate(..) => {
                    if let Some(SVGTransform::Rotate(n, x, y)) = rounded.get(i + 1) {
                        if *n != 180.0 && *n != -180.0 && *n != 0.0 && *x == 0.0 && *y == 0.0 {
                            log::debug!("merging translate and rotate");
                            let translate = &raw[i];
                            let rotate = &raw[i + 1];
                            optimized
                                .push(SVGTransform::merge_translate_and_rotate(translate, rotate));
                            skip = true;
                            continue;
                        }
                    }
                    optimized.push(item.clone());
                }
                SVGTransform::Matrix(_) => unreachable!(),
                _ => optimized.push(item.clone()),
            }
        }

        if optimized.is_empty() {
            vec![SVGTransform::Scale(1.0, 1.0)]
        } else {
            optimized
        }
    }

    /// Converts the transform to a 3D matrix.
    pub fn to_matrix(&self) -> Option<Matrix3d<Number>> {
        let mut matrix = Matrix3d::identity();
        for transform in &self.0 {
            let transform = if let SVGTransform::Rotate(angle, 0.0, 0.0) = transform {
                Transform::Matrix3d(Matrix3d::rotate(0.0, 0.0, 1.0, angle.to_radians()))
            } else {
                transform.clone().into()
            };
            if let Some(m) = transform.to_matrix() {
                matrix = m.multiply(&matrix);
            } else {
                return None;
            }
        }
        Some(matrix)
    }

    /// Attempts to convert the matrix to 2D.
    pub fn to_matrix_2d(&self) -> Option<Matrix<Number>> {
        self.to_matrix().and_then(|m| m.to_matrix2d())
    }
}

impl From<SVGTransformList> for TransformList {
    fn from(val: SVGTransformList) -> Self {
        TransformList(val.0.into_iter().map(Into::into).collect())
    }
}

impl TryFrom<&TransformList> for SVGTransformList {
    type Error = ();

    fn try_from(value: &TransformList) -> Result<Self, Self::Error> {
        let list: Result<Vec<_>, _> = value.0.iter().map(TryInto::try_into).collect();
        Ok(Self(list?))
    }
}

#[cfg(feature = "parse")]
impl<'input> Parse<'input> for SVGTransformList {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let mut result = Vec::new();
        loop {
            input.skip_whitespace();
            input.try_parse(Parser::expect_comma).ok();
            input.skip_whitespace();
            if let Ok(item) = input.try_parse(SVGTransform::parse) {
                result.push(item);
            } else {
                return Ok(Self(result));
            }
        }
    }
}
#[cfg(feature = "serialize")]
impl ToValue for SVGTransformList {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let mut items = self.0.iter();
        if let Some(first) = items.next() {
            first.write_value(dest)?;
        }
        for item in items {
            item.write_value(dest)?;
        }
        Ok(())
    }
}

impl Precision {
    /// Rounds a number to a given precision
    fn round_arg(precision: i32, data: &mut f32) {
        *data = if (1..20).contains(&precision) {
            Self::smart_round(precision, *data)
        } else {
            data.round()
        }
    }

    /// Rounds a number to a given precision
    fn smart_round(precision: i32, data: f32) -> f32 {
        let tolerance = Self::to_fixed(0.1_f32.powi(precision), precision);
        if Self::to_fixed(data, precision) == data {
            data
        } else {
            let rounded = Self::to_fixed(data, precision - 1);
            if Self::to_fixed((rounded - data).abs(), precision + 1) >= tolerance {
                Self::to_fixed(data, precision)
            } else {
                rounded
            }
        }
    }

    fn to_fixed(data: f32, precision: i32) -> f32 {
        let pow = 10.0_f32.powi(precision);
        f32::round(data * pow) / pow
    }
}
