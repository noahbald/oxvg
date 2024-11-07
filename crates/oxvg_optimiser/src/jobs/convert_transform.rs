use std::rc::Rc;

use itertools::Itertools;
use lightningcss::properties::transform::{Matrix, Matrix3d, Transform, TransformList};
use markup5ever::{local_name, LocalName};
use oxvg_selectors::Element;
use oxvg_style::{SVGStyle, SVGStyleID, Style};
use serde::Deserialize;

use crate::{
    utils::transform::{self, Precision},
    Context, Job,
};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertTransform {
    convert_to_shorts: Option<bool>,
    deg_precision: Option<i32>,
    float_precision: Option<i32>,
    transform_precision: Option<i32>,
    matrix_to_transform: Option<bool>,
    short_translate: Option<bool>,
    short_scale: Option<bool>,
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
    short_translate: bool,
    short_scale: bool,
    short_rotate: bool,
    remove_useless: bool,
    collapse_into_one: bool,
}

impl Job for ConvertTransform {
    fn use_style(&self, node: &Rc<rcdom::Node>) -> bool {
        use rcdom::NodeData::Element;

        let Element { attrs, .. } = &node.data else {
            return false;
        };

        attrs.borrow().iter().any(|attr| {
            matches!(
                attr.name.local,
                local_name!("transform")
                    | local_name!("gradientTransform")
                    | local_name!("patternTransform")
            )
        })
    }

    fn run(&self, node: &Rc<rcdom::Node>, context: &Context) {
        let element = Element::new(node.clone());

        if let Some(transform) = context.style.attr.get(&SVGStyleID::Transform) {
            self.transform_attr(transform, &local_name!("transform"), &element);
        }

        if let Some(gradient_transform) = context.style.attr.get(&SVGStyleID::GradientTransform) {
            self.transform_attr(
                gradient_transform,
                &local_name!("patternTransform"),
                &element,
            );
        }

        if let Some(pattern_transform) = context.style.attr.get(&SVGStyleID::PatternTransform) {
            self.transform_attr(
                pattern_transform,
                &local_name!("patternTransform"),
                &element,
            );
        }
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
            short_translate: self.short_translate.unwrap_or(true),
            short_scale: self.short_scale.unwrap_or(true),
            short_rotate: self.short_rotate.unwrap_or(true),
            remove_useless: self.remove_useless.unwrap_or(true),
            collapse_into_one: self.collapse_into_one.unwrap_or(true),
        }
    }

    fn transform_attr(&self, value: &Style, name: &LocalName, element: &Element) {
        if let Some(value) = self.transform(value) {
            if value.is_empty() {
                log::debug!("transform_attr: removing {name}");
                element.remove_attr(name);
            } else {
                log::debug!("transform_attr: updating {name}");
                element.set_attr(name, value.into());
            }
        }
    }

    fn to_matrix(list: &TransformList) -> Option<Matrix<f32>> {
        let mut matrix = Matrix3d::identity();
        for transform in &list.0 {
            let transform = if let Transform::Rotate(angle) = transform {
                &Transform::Matrix3d(Matrix3d::rotate(0.0, 0.0, 1.0, angle.to_radians()))
            } else {
                transform
            };
            if let Some(m) = transform.to_matrix() {
                matrix = m.multiply(&matrix);
            } else {
                return None;
            }
        }
        matrix.to_matrix2d()
    }

    fn transform(&self, value: &Style) -> Option<String> {
        let Style::Static(SVGStyle::Transform(transform, _)) = value else {
            unreachable!("dynamic or non-transform style provided");
        };
        let inner = self.inner().define_precision(transform, self);

        let data = if inner.collapse_into_one && transform.0.len() > 1 {
            if let Some(matrix) = Self::to_matrix(transform) {
                log::debug!("collapsing transform to matrix");
                &TransformList(vec![Transform::Matrix(matrix)])
            } else {
                transform
            }
        } else {
            transform
        };
        let Ok(mut data) = data
            .0
            .iter()
            .map(transform::Transform::try_from)
            .collect::<Result<Vec<_>, ()>>()
        else {
            log::debug!("failed to convert lightningcss transform to our transform");
            return None;
        };
        log::debug!(r#"working with data "{:?}""#, data);

        let data = inner.convert_to_shorts(&mut data);
        log::debug!(r#"converted transforms to short "{:?}""#, data);
        let data = inner.remove_useless(data);
        log::debug!(r#"removed useless transform "{:?}""#, data);
        Some(
            data.into_iter()
                .map(|transform| transform.to_string())
                .join(""),
        )
    }
}

impl Inner {
    fn define_precision(mut self, data: &TransformList, outer: &ConvertTransform) -> Self {
        let matrix_data = data
            .0
            .iter()
            .filter_map(|t| match t {
                Transform::Matrix(matrix) => Some(matrix),
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

    fn convert_to_shorts(&self, data: &mut [transform::Transform]) -> Vec<transform::Transform> {
        let precision = Precision {
            float: self.float_precision,
            deg: self.deg_precision,
            transform: self.transform_precision,
        };
        log::debug!("converting with precision: {:?}", precision);
        if !self.convert_to_shorts {
            data.iter_mut()
                .for_each(|transform| transform.round(&precision));
        }

        let shorts: Vec<_> = data
            .iter()
            .flat_map(|transform| {
                if self.matrix_to_transform {
                    if let transform::Transform::Matrix(matrix) = transform {
                        return matrix.to_transform(&precision);
                    }
                }
                vec![transform.clone()]
            })
            .map(|mut transform| {
                transform.round(&precision);
                match &mut transform {
                    transform::Transform::Translate((_, y))
                        if self.short_translate && *y == Some(0.0) =>
                    {
                        log::debug!(r#"shortened translate"#);
                        *y = None;
                    }
                    transform::Transform::Scale((x, y)) if self.short_scale && *y == Some(*x) => {
                        log::debug!(r#"shortened scale"#);
                        *y = None;
                    }
                    _ => {}
                }
                transform
            })
            .collect();
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
                let transform::Transform::Translate((start_x, Some(start_y))) = shorts[i] else {
                    result.push(start.clone());
                    continue;
                };
                let transform::Transform::Rotate((deg, None)) = shorts[i + 1] else {
                    result.push(start.clone());
                    continue;
                };
                let transform::Transform::Translate((x, Some(y))) = shorts[i + 2] else {
                    result.push(start.clone());
                    continue;
                };
                if -start_x != x || start_y != y {
                    log::debug!("start and end not equivalent");
                    result.push(start.clone());
                    continue;
                }
                result.push(transform::Transform::Rotate((
                    deg,
                    Some((start_x, start_y)),
                )));
                skip = 2;
            }
            log::debug!(r#"shortened rotates: "{:?}""#, result);
            result
        } else {
            shorts
        }
    }

    fn remove_useless(&self, data: Vec<transform::Transform>) -> Vec<transform::Transform> {
        if !self.remove_useless {
            return data;
        }

        data.into_iter()
            .filter(|item| {
                !matches!(
                    item,
                    transform::Transform::Translate((0.0, None | Some(0.0)))
                        | transform::Transform::Rotate((0.0, _))
                        | transform::Transform::SkewX(0.0)
                        | transform::Transform::SkewY(0.0)
                        | transform::Transform::Scale((1.0, None))
                        | transform::Transform::Matrix(transform::Matrix(Matrix {
                            a: 1.0,
                            b: 0.0,
                            c: 0.0,
                            d: 1.0,
                            e: 0.0,
                            f: 0.0
                        }))
                )
            })
            .collect()
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn add_attributes_to_svg_element() -> anyhow::Result<()> {
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
