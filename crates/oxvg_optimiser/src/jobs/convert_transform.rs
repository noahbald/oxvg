use lightningcss::{
    printer::PrinterOptions,
    properties::transform::{Matrix, TransformList},
    traits::ToCss,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    style::{
        Id, Precision, PresentationAttr, PresentationAttrId, SVGTransform, SVGTransformList,
        Static, Style,
    },
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
/// Merge transforms and convert to shortest form.
///
/// # Correctness
///
/// Rounding errors may cause slight changes in visual appearance.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct ConvertTransform {
    /// Whether to convert transforms to their shorthand alternative.
    #[serde(default = "default_convert_to_shorts")]
    pub convert_to_shorts: bool,
    /// Number of decimal places to round degrees to, for `rotate` and `skew`.
    ///
    /// Some of the precision may also be lost during serialization.
    #[serde(default = "Option::default")]
    pub deg_precision: Option<i32>,
    /// Number of decimal places to round to, for `rotate`'s origin and `translate`.
    #[serde(default = "default_float_precision")]
    pub float_precision: i32,
    /// Number of decimal places to round to, for `scale`.
    #[serde(default = "default_transform_precision")]
    pub transform_precision: i32,
    /// Whether to convert matrices into transforms.
    #[serde(default = "default_matrix_to_transform")]
    pub matrix_to_transform: bool,
    /// Whether to remove redundant arguments from `translate` (e.g. `translate(10 0)` -> `transflate(10)`).
    #[serde(default = "default_short_rotate")]
    pub short_rotate: bool,
    /// Whether to remove redundant transforms (e.g. `translate(0)`).
    #[serde(default = "default_remove_useless")]
    pub remove_useless: bool,
    /// Whether to merge transforms.
    #[serde(default = "default_collapse_into_one")]
    pub collapse_into_one: bool,
}

impl Default for ConvertTransform {
    fn default() -> Self {
        Self {
            convert_to_shorts: default_convert_to_shorts(),
            deg_precision: None,
            float_precision: default_float_precision(),
            transform_precision: default_transform_precision(),
            matrix_to_transform: default_matrix_to_transform(),
            short_rotate: default_short_rotate(),
            remove_useless: default_remove_useless(),
            collapse_into_one: default_collapse_into_one(),
        }
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for ConvertTransform {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(PrepareOutcome::use_style)
    }

    fn use_style(&self, element: &E) -> bool {
        element.attributes().into_iter().any(|attr| {
            matches!(
                attr.local_name().as_ref(),
                "transform" | "gradientTransform" | "patternTransform"
            )
        })
    }

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        if let Some(transform) = context
            .computed_styles
            .attr
            .get(&PresentationAttrId::Transform)
        {
            self.transform_attr(transform, "transform", element);
        }

        if let Some(gradient_transform) = context
            .computed_styles
            .attr
            .get(&PresentationAttrId::GradientTransform)
        {
            self.transform_attr(gradient_transform, "patternTransform", element);
        }

        if let Some(pattern_transform) = context
            .computed_styles
            .attr
            .get(&PresentationAttrId::PatternTransform)
        {
            self.transform_attr(pattern_transform, "patternTransform", element);
        }
        Ok(())
    }
}

impl ConvertTransform {
    fn define_precision(&self, data: &SVGTransformList) -> (i32, i32) {
        let matrix_data = data
            .0
            .iter()
            .filter_map(|t| match t {
                SVGTransform::Matrix(matrix) => Some(matrix),
                _ => None,
            })
            .flat_map(|m| [m.a, m.b, m.c, m.d, m.e, m.f]);
        let mut number_of_digits = 0;
        let mut number_of_decimals = 0;
        let mut transform_precision = self.transform_precision;
        let mut is_matrix_data = false;
        for arg in matrix_data {
            is_matrix_data = true;
            let arg_string = format!("{arg}");
            // 123.45 -> 5
            let arg_total_digits = arg_string
                .chars()
                .filter(|char| char.is_numeric())
                .count()
                .try_into()
                .expect("number too long!");
            // 123.45 -> 2
            let arg_decimal_digits = arg_string
                .chars()
                .skip_while(|char| char != &'.')
                .filter(|char| char.is_numeric())
                .count()
                .try_into()
                .expect("number too long!");
            if arg_total_digits > number_of_digits {
                number_of_digits = arg_total_digits;
            }
            if arg_decimal_digits > number_of_decimals {
                number_of_decimals = arg_decimal_digits;
            }
        }
        if is_matrix_data && number_of_decimals < transform_precision {
            transform_precision = number_of_decimals;
        }
        if !is_matrix_data {
            number_of_digits = self.transform_precision;
        }

        let deg_precision = match self.deg_precision {
            Some(value) => value,
            None => i32::max(0, i32::min(self.float_precision, number_of_digits - 2)),
        };

        (deg_precision, transform_precision)
    }

    fn transform_attr<'arena, E: Element<'arena>>(&self, value: &Style, name: &str, element: &E) {
        log::debug!("transform_attr: found {name} to transform");
        if value.is_unparsed() {
            element.remove_attribute_local(&name.into());
            return;
        }
        if let Some(transform) = self.transform(value) {
            if transform.0.is_empty() {
                log::debug!("transform_attr: removing {name}");
                element.remove_attribute_local(&name.into());
            } else {
                log::debug!("transform_attr: updating {name}");
                let value = match value.inner().id() {
                    Id::Attr(PresentationAttrId::Transform) => {
                        transform.to_css_string(PrinterOptions {
                            minify: true,
                            ..Default::default()
                        })
                    }
                    Id::Attr(
                        PresentationAttrId::GradientTransform
                        | PresentationAttrId::PatternTransform,
                    ) => Into::<TransformList>::into(transform).to_css_string(PrinterOptions {
                        minify: true,
                        ..Default::default()
                    }),
                    _ => return,
                }
                .unwrap_or_default();
                element.set_attribute_local(name.into(), value.into());
            }
        }
    }

    fn transform(&self, value: &Style) -> Option<SVGTransformList> {
        let transform = match value {
            Style::Static(Static::Attr(
                PresentationAttr::Transform(transform)
                | PresentationAttr::GradientTransform(transform)
                | PresentationAttr::PatternTransform(transform),
            )) => transform.clone(),
            Style::Static(Static::Attr(PresentationAttr::Unparsed(_))) => return None,
            _ => unreachable!("dynamic or non-transform style provided"),
        };

        let mut data = if self.collapse_into_one && transform.0.len() > 1 {
            if let Some(matrix) = transform.to_matrix_2d() {
                log::debug!("collapsing transform to matrix");
                SVGTransformList(vec![SVGTransform::Matrix(matrix)])
            } else {
                transform
            }
        } else {
            transform
        };
        log::debug!(r#"working with data "{:?}""#, data);

        let data = self.convert_to_shorts(&mut data);
        log::debug!(r#"converted transforms to short "{:?}""#, data);
        let data = self.remove_useless(data);
        log::debug!(r#"removed useless transform "{:?}""#, data);
        Some(SVGTransformList(data))
    }

    fn remove_useless(&self, data: Vec<SVGTransform>) -> Vec<SVGTransform> {
        if !self.remove_useless {
            return data;
        }

        data.into_iter()
            .filter(|item| {
                !matches!(
                    item,
                    SVGTransform::Translate(0.0, 0.0)
                        | SVGTransform::Rotate(0.0, _, _)
                        | SVGTransform::SkewX(0.0)
                        | SVGTransform::SkewY(0.0)
                        | SVGTransform::Scale(1.0, 1.0)
                        | SVGTransform::Matrix(Matrix {
                            a: 1.0,
                            b: 0.0,
                            c: 0.0,
                            d: 1.0,
                            e: 0.0,
                            f: 0.0
                        })
                )
            })
            .collect()
    }

    fn convert_to_shorts(&self, data: &mut SVGTransformList) -> Vec<SVGTransform> {
        let (deg_precision, transform_precision) = self.define_precision(data);
        let precision = Precision {
            float: self.float_precision,
            deg: deg_precision,
            transform: transform_precision,
        };
        log::debug!("converting with precision: {:?}", precision);
        if !self.convert_to_shorts {
            data.0
                .iter_mut()
                .for_each(|transform| transform.round(&precision));
        }

        let shorts: Vec<_> = if self.matrix_to_transform {
            data.0
                .iter()
                .flat_map(|transform| transform.matrix_to_transform(&precision))
                .map(|mut transform| {
                    transform.round(&precision);
                    transform
                })
                .collect()
        } else {
            data.0
                .clone()
                .into_iter()
                .map(|mut transform| {
                    transform.round(&precision);
                    transform
                })
                .collect()
        };
        if self.short_rotate && shorts.len() >= 3 {
            let mut result = vec![];
            let mut skip = 0;
            for i in 0..shorts.len() {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let start = &shorts[i];
                if i >= shorts.len() - 2 {
                    result.push(start.clone());
                    continue;
                }
                let SVGTransform::Translate(start_x, start_y) = shorts[i] else {
                    result.push(start.clone());
                    continue;
                };
                let SVGTransform::Rotate(deg, 0.0, 0.0) = shorts[i + 1] else {
                    result.push(start.clone());
                    continue;
                };
                let SVGTransform::Translate(end_x, end_y) = shorts[i + 2] else {
                    result.push(start.clone());
                    continue;
                };
                if start_x != -end_x || start_y != -end_y {
                    log::debug!("start and end not equivalent");
                    result.push(start.clone());
                    continue;
                }
                result.push(SVGTransform::Rotate(deg, start_x, start_y));
                skip = 2;
            }
            log::debug!(r#"shortened rotates: "{:?}""#, result);
            result
        } else {
            shorts
        }
    }
}

const fn default_convert_to_shorts() -> bool {
    true
}

const fn default_float_precision() -> i32 {
    3
}

const fn default_transform_precision() -> i32 {
    5
}

const fn default_matrix_to_transform() -> bool {
    true
}

const fn default_short_rotate() -> bool {
    true
}

const fn default_remove_useless() -> bool {
    true
}

const fn default_collapse_into_one() -> bool {
    true
}

#[test]
#[allow(clippy::too_many_lines)]
fn convert_transform() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300">
    <rect width="10" height="20" transform="matrix(0.707 -0.707 0.707 0.707 255.03 111.21)"/>
    <rect width="10" height="20" transform="matrix(1 0 0 1 50 90),matrix(0.707 -0.707 0.707 0.707 0 0) ,matrix(1 0 0 1 130 160)"/>
    <rect width="10" height="20" transform="translate(50 90) , rotate(-45)   translate(130 160)"/>
    <rect width="10" height="20" transform="matrix(0.707 -0.707 0.707 0.707 255.03 111.21) scale(2)"/>
    <rect width="10" height="20" transform="matrix(0.707 -0.707 0.707 0.707 255.03 111.21) skewX(45)"/>
    <rect width="10" height="20" transform="matrix( 0.707 -0.707 0.707 0.707 255.03 111.21 ) skewY( 45 )"/>
    <rect width="10" height="20" transform="matrix(1 0 1 1 0 0)"/>
    <rect width="10" height="20" transform="matrix(1.25,0,0,-1.25,0,56.26) scale(1,-1)"/>
    <rect width="10" height="20" transform="matrix(1.25,0,0,-1.25,0,56.26) matrix(0.1325312,0,0,-0.1325312,-31.207631,89.011662)"/>
    <rect width="10" height="20" transform="matrix(1 0 0 -1 0 0)"/>
    <rect width="10" height="20" transform="matrix(-1 0 0 1 0 0)"/>
    <rect width="10" height="20" transform="matrix(0 1-1 0 0 0)"/>
    <rect width="10" height="20" transform="matrix(0-1 1 0 0 0)"/>
    <rect width="10" height="20" transform="matrix(0.707 -0.707 -0.707 -0.707 0 0)"/>
    <rect width="10" height="20" transform="matrix(-0.707 0.707 0.707 0.707 0 0)"/>
    <rect width="10" height="20" transform="matrix(-0.707 0.707 -0.707 -0.707 0 0)"/>
    <rect width="10" height="20" transform="matrix(0.707 0.707 -0.707 0.707 0 0)"/>
    <rect width="10" height="20" transform="matrix(.647 -.647 -.6443 -.6443 0 0)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <g transform="translate(50 0) scale(2 2)"/>
    <g transform="translate(50) scale(2 2)"/>
    <g transform="translate(10 20) rotate(45) translate(-10-20)"/>
    <g transform="scale(2) translate(10 20) rotate(45) translate(-10-20)"/>
    <g transform="rotate(15) scale(2 1)"/>
    <g transform="scale(2 1) rotate(15)"/>
    <g transform="translate(10 20) rotate(45) translate(-10-20) scale(2)"/>
    <g transform="translate(15, 3) translate(13) rotate(47 39.885486 39.782373)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <g transform="matrix(1 0 0 1 50 100)"/>
    <g transform="matrix(0.5 0 0 2 0 0)"/>
    <g transform="matrix(.707-.707.707.707 0 0)"/>
    <g transform="matrix(1 0 0.466 1 0 0)"/>
    <g transform="matrix(1 0.466 0 1 0 0)"/>
    <g transform="matrix(1 0 0 1 50 90) matrix(1 0 0 1 60 20) matrix(1 0 0 1 20 40)"/>
    <g transform="matrix(-0.10443115234375 0 0 -0.10443115234375 182.15 61.15)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <g transform=""/>
    <g transform="translate(0)"/>
    <g transform="translate(0 0)"/>
    <g transform="translate(0 50)"/>
    <g transform="scale(1)"/>
    <g transform="scale(1 2)"/>
    <g transform="rotate(0)"/>
    <g transform="rotate(0 100 100)"/>
    <g transform="skewX(0)"/>
    <g transform="skewY(0)"/>
    <g transform="translate(0,-100) translate(0,100)"/>
    <g transform="rotate(45, 34, 34"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect x="-45" y="-77" height="3" width="8" transform="matrix(0,-1,-1,0,0,0)" />
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": { "degPrecision": 1 } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
  <text y="32" transform="matrix(1.0000002 0 0 1 0 0)">uwu</text>
  <text y="64" transform="matrix(1 0 0 1 0.00002 0)">uwu</text>
  <text y="96" transform="matrix(0.9999999847691 1.745329243133368e-4 -1.745329243133368e-4 0.9999999847691 0 0)">uwu</text>
  <text y="128" transform="matrix(1.0000002 0 0 1 0.00002 0)">uwu</text>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": { "degPrecision": 3 } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <text x="-32" y="32" transform="matrix(-1,0,-0.3,0.9,0,0)">uwu</text>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500" viewBox="-100 -100 100 100">
    <rect x="0" y="0" width="10" height="20" transform="matrix(1,0,0,1,3,0)"/>
    <rect x="0" y="0" width="10" height="20" transform="matrix(1,0,0,1,3,3)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500" viewBox="-100 -100 100 100">
    <rect x="0" y="0" width="10" height="20" transform="matrix(-1,0,0,-1,5,7)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500" viewBox="-100 -100 100 100">
    <rect x="0" y="0" width="10" height="20" transform="matrix(-1,0,0,-1,0 0)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-50 -50 100 100">
    <rect x="0" y="0" width="10" height="20" transform="matrix(1.93185,0.51764,-0.25882,0.96593,0,0)"/>
    <rect x="-20" y="-20" width="10" height="20" transform="matrix(0.85606,0.66883,-0.25882,0.96593,0,0)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": { "degPrecision":4, "floatPrecision":6, "transformPrecision":8} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
    <rect x="20" y="30" width="40" height="50" transform="matrix(-1,-4.371139e-8,4.371139e-8,-1,139.2007,136.8)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 100 150">
    <rect x="0" y="10" width="5" height="8" fill="red" transform="translate(5,70) scale(.4 0)"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertTransform": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500" viewBox="0 0 480 360">
    <!-- ignore inherited styles on children -->
    <g transform="translate(30,-10)">
      <rect x="0" y="0" width="10" height="20"/>
    </g>
</svg>"#
        ),
    )?);

    Ok(())
}
