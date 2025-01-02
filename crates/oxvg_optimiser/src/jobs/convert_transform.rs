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
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct ConvertTransform {
    convert_to_shorts: Option<bool>,
    // NOTE: Some of the precision will be thrown out by lightningcss' serialization
    deg_precision: Option<i32>,
    float_precision: Option<i32>,
    transform_precision: Option<i32>,
    matrix_to_transform: Option<bool>,
    short_rotate: Option<bool>,
    remove_useless: Option<bool>,
    collapse_into_one: Option<bool>,
}

#[allow(clippy::struct_excessive_bools)]
struct Inner {
    convert_to_shorts: bool,
    deg_precision: i32,
    float_precision: i32,
    transform_precision: i32,
    matrix_to_transform: bool,
    short_rotate: bool,
    remove_useless: bool,
    collapse_into_one: bool,
}

impl<E: Element> Job<E> for ConvertTransform {}

impl<E: Element> Visitor<E> for ConvertTransform {
    type Error = String;

    fn prepare(
        &mut self,
        _document: &E,
        _context_flags: &oxvg_ast::visitor::ContextFlags,
    ) -> PrepareOutcome {
        PrepareOutcome::use_style
    }

    fn use_style(&self, element: &E) -> bool {
        element.attributes().iter().any(|attr| {
            matches!(
                attr.local_name().as_ref(),
                "transform" | "gradientTransform" | "patternTransform"
            )
        })
    }

    fn element(&mut self, element: &mut E, context: &Context<E>) -> Result<(), String> {
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
    fn inner(&self) -> Inner {
        Inner {
            convert_to_shorts: self.convert_to_shorts.unwrap_or(true),
            deg_precision: self.deg_precision.unwrap_or(3),
            float_precision: self.float_precision.unwrap_or(3),
            transform_precision: self.transform_precision.unwrap_or(5),
            matrix_to_transform: self.matrix_to_transform.unwrap_or(true),
            short_rotate: self.short_rotate.unwrap_or(true),
            remove_useless: self.remove_useless.unwrap_or(true),
            collapse_into_one: self.collapse_into_one.unwrap_or(true),
        }
    }

    fn transform_attr(&self, value: &Style, name: &str, element: &impl Element) {
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
                        transform.to_css_string(PrinterOptions::default())
                    }
                    Id::Attr(
                        PresentationAttrId::GradientTransform
                        | PresentationAttrId::PatternTransform,
                    ) => Into::<TransformList>::into(transform)
                        .to_css_string(PrinterOptions::default()),
                    _ => return,
                }
                .unwrap_or_default();
                element.set_attribute_local(name.into(), value.into());
            }
        }
    }

    fn transform(&self, value: &Style) -> Option<SVGTransformList> {
        let transform = match value {
            Style::Static(Static::Attr(PresentationAttr::Transform(transform))) => {
                transform.clone()
            }
            Style::Static(Static::Attr(
                PresentationAttr::GradientTransform(transform)
                | PresentationAttr::PatternTransform(transform),
            )) => match transform.try_into() {
                Ok(transform) => transform,
                Err(()) => return None,
            },
            Style::Static(Static::Attr(PresentationAttr::Unparsed(_))) => return None,
            _ => unreachable!("dynamic or non-transform style provided"),
        };
        let inner = self.inner().define_precision(&transform, self);

        let mut data = if inner.collapse_into_one && transform.0.len() > 1 {
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

        let data = inner.convert_to_shorts(&mut data);
        log::debug!(r#"converted transforms to short "{:?}""#, data);
        let data = inner.remove_useless(data);
        log::debug!(r#"removed useless transform "{:?}""#, data);
        Some(SVGTransformList(data))
    }
}

impl Inner {
    fn define_precision(mut self, data: &SVGTransformList, outer: &ConvertTransform) -> Self {
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
        if outer.deg_precision.is_none() {
            self.deg_precision = i32::max(0, i32::min(self.float_precision, number_of_digits - 2));
        }

        self.transform_precision = transform_precision;
        self
    }

    fn convert_to_shorts(&self, data: &mut SVGTransformList) -> Vec<SVGTransform> {
        let precision = Precision {
            float: self.float_precision,
            deg: self.deg_precision,
            transform: self.transform_precision,
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

    Ok(())
}
