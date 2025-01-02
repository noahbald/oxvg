use core::ops::Mul;
use std::f64;

use lightningcss::{
    printer::PrinterOptions,
    properties::{
        svg::{self, SVGPaint},
        transform::{Matrix, TransformList},
        Property, PropertyId,
    },
    traits::ToCss,
    values::{length::LengthValue, percentage::DimensionPercentage},
    vendor_prefix::VendorPrefix,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    get_computed_styles_factory,
    style::{self, ComputedStyles, Id, PresentationAttr, PresentationAttrId, Static, Style},
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::{collections, regex::REFERENCES_URL};
use oxvg_derive::OptionalDefault;
use oxvg_path::{command::Data, convert, Path};
use serde::Deserialize;

#[derive(Deserialize, Default, Clone, Debug, OptionalDefault)]
#[serde(rename_all = "camelCase")]
/// Apply transformations to the path data
pub struct ApplyTransforms {
    transform_precision: Option<f64>,
    apply_transforms_stroked: Option<bool>,
}

impl<E: Element> Visitor<E> for ApplyTransforms {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &ContextFlags) -> PrepareOutcome {
        PrepareOutcome::use_style
    }

    fn use_style(&self, element: &E) -> bool {
        element.get_attribute_local(&"d".into()).is_some()
    }

    fn element(&mut self, element: &mut E, context: &Context<E>) -> Result<(), String> {
        let d_localname = "d".into();
        let Some(path) = element.get_attribute_local(&d_localname) else {
            log::debug!("run: path has no d");
            return Ok(());
        };
        let Ok(mut path) = Path::parse(path.into()) else {
            log::debug!("run: failed to parse path");
            return Ok(());
        };

        for attr in element.attributes().iter() {
            if attr.local_name() == "id".into() || attr.local_name() == "style".into() {
                log::debug!("run: element has id");
                return Ok(());
            }

            let is_reference_prop =
                collections::REFERENCES_PROPS.contains(attr.local_name().as_ref());
            if is_reference_prop && REFERENCES_URL.captures(attr.value().as_ref()).is_some() {
                log::debug!("run: element has reference");
                return Ok(());
            }
        }

        let Some(Style::Static(Static::Attr(PresentationAttr::Transform(transform)))) = context
            .computed_styles
            .attr
            .get(&PresentationAttrId::Transform)
        else {
            log::debug!("run: element has no transform");
            return Ok(());
        };
        if transform.0.is_empty() {
            log::debug!("run: cannot handle empty transform");
            return Ok(());
        }
        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);
        if let Some(css_transform) = get_computed_styles!(Transform(VendorPrefix::None)) {
            if css_transform.is_static()
                && css_transform.to_css_string(false)
                    != transform.to_css_string(PrinterOptions::default()).ok()
            {
                log::debug!("run: another transform is applied to this element");
                return Ok(());
            }
        };

        let stroke = get_computed_styles!(Stroke);
        if stroke.is_some_and(Style::is_dynamic) {
            log::debug!("run: cannot handle dynamic stroke");
            return Ok(());
        }
        let stroke = stroke.map(Style::inner);

        let stroke_width = get_computed_styles!(StrokeWidth);
        if stroke_width.is_some_and(Style::is_dynamic) {
            log::debug!("run: cannot handle dynamic stroke_width");
            return Ok(());
        }
        let stroke_width = stroke_width.map(|s| match s.inner() {
            Static::Attr(PresentationAttr::StrokeWidth(p))
            | Static::Css(Property::StrokeWidth(p)) => p,
            _ => unreachable!(),
        });
        let css_transform: TransformList = transform.clone().into();
        let Some(matrix) = css_transform.to_matrix() else {
            log::debug!("run: cannot get matrix");
            return Ok(());
        };
        let Some(matrix) = matrix.to_matrix2d() else {
            log::debug!("run: cannot handle matrix");
            return Ok(());
        };
        let matrix = matrix32_to_slice(&matrix);

        if let Some(
            Static::Attr(PresentationAttr::Stroke(stroke)) | Static::Css(Property::Stroke(stroke)),
        ) = stroke
        {
            if self.apply_stroked(
                &matrix,
                &context.computed_styles,
                &stroke,
                stroke_width.as_ref(),
                element,
            ) {
                return Ok(());
            }
        }

        apply_matrix_to_path_data(&mut path, &matrix);
        element.set_attribute_local(
            d_localname,
            convert::cleanup_unpositioned(&path).to_string().into(),
        );
        log::debug!("new d <- {path}");
        element.remove_attribute_local(&"transform".into());
        Ok(())
    }
}

impl ApplyTransforms {
    #[allow(clippy::float_cmp, clippy::cast_possible_truncation)]
    fn apply_stroked(
        &self,
        matrix: &[f64; 6],
        style: &ComputedStyles,
        stroke: &SVGPaint,
        stroke_width: Option<&DimensionPercentage<LengthValue>>,
        element: &impl Element,
    ) -> bool {
        if matches!(stroke, SVGPaint::None) {
            return false;
        }
        if self.apply_transforms_stroked.unwrap_or(false) {
            log::debug!("apply_stroked: not applying transformed stroke");
            return true;
        }
        if (matrix[0] != matrix[3] || matrix[1] != -matrix[2])
            && (matrix[0] != -matrix[3] || matrix[1] != matrix[2])
        {
            log::debug!("apply_stroked: stroke cannot be applied with disproportional scale/skew");
            return true;
        }

        let vector_effect = element.get_attribute_local(&"vector-effect".into());
        if vector_effect
            .as_ref()
            .is_some_and(|v| v.as_ref() == "non-scaling-stroke")
        {
            return false;
        }

        let mut scale = f64::sqrt((matrix[0] * matrix[0]) + (matrix[1] * matrix[1])); // hypot
        if let Some(transform_precision) = self.transform_precision {
            scale = f64::round(scale * transform_precision) / transform_precision;
        }
        if scale == 1.0 {
            return false;
        }

        let mut stroke_width = match stroke_width {
            Some(value) => value.clone(),
            None => DimensionPercentage::px(1.0),
        };
        stroke_width = stroke_width.mul(scale as f32);
        if let Ok(value) = stroke_width.to_css_string(PrinterOptions::default()) {
            element.set_attribute_local("stroke-width".into(), value.trim_end_matches("px").into());
        }

        if let Some(
            Static::Attr(PresentationAttr::StrokeDashoffset(l))
            | Static::Css(Property::StrokeDashoffset(l)),
        ) = style
            .attr
            .get(&PresentationAttrId::StrokeDashoffset)
            .map(Style::inner)
        {
            if let Ok(value) = l
                .clone()
                .mul(scale as f32)
                .to_css_string(PrinterOptions::default())
            {
                element.set_attribute_local("stroke-dashoffset".into(), value.into());
            };
        };

        if let Some(
            Static::Attr(PresentationAttr::StrokeDasharray(svg::StrokeDasharray::Values(v)))
            | Static::Css(Property::StrokeDasharray(svg::StrokeDasharray::Values(v))),
        ) = style
            .attr
            .get(&PresentationAttrId::StrokeDasharray)
            .map(Style::inner)
        {
            let value = v
                .clone()
                .into_iter()
                .map(|l| l.mul(scale as f32))
                .collect::<Vec<_>>();
            if let Ok(value) =
                svg::StrokeDasharray::Values(value).to_css_string(PrinterOptions::default())
            {
                element.set_attribute_local("stroke-dasharray".into(), value.into());
            }
        }

        false
    }
}

fn matrix32_to_slice(matrix: &Matrix<f32>) -> [f64; 6] {
    [
        f64::from(matrix.a),
        f64::from(matrix.b),
        f64::from(matrix.c),
        f64::from(matrix.d),
        f64::from(matrix.e),
        f64::from(matrix.f),
    ]
}

#[allow(clippy::too_many_lines)]
fn apply_matrix_to_path_data(path_data: &mut Path, matrix: &[f64; 6]) {
    log::debug!("applying matrix: {:?}", matrix);
    let mut start = [0.0; 2];
    let mut cursor = [0.0; 2];
    if let Some(data) = path_data.0.get_mut(0) {
        if let Data::MoveBy(args) = data {
            *data = Data::MoveTo(*args);
        }
    }

    path_data.0.iter_mut().for_each(|data| {
        if let Data::Implicit(_) = data {
            *data = data.as_explicit().clone();
        };
        match data {
            Data::HorizontalLineTo(args) => *data = Data::LineTo([args[0], cursor[1]]),
            Data::HorizontalLineBy(args) => *data = Data::LineBy([args[0], 0.0]),
            Data::VerticalLineTo(args) => *data = Data::LineTo([cursor[0], args[0]]),
            Data::VerticalLineBy(args) => *data = Data::LineBy([0.0, args[0]]),
            _ => {}
        };
        match data {
            Data::MoveTo(args) => {
                cursor[0] = args[0];
                cursor[1] = args[1];
                start[0] = cursor[0];
                start[1] = cursor[1];
                *args = transform_absolute_point(matrix, args[0], args[1]);
            }
            Data::MoveBy(args) => {
                cursor[0] += args[0];
                cursor[1] += args[1];
                start[0] = cursor[0];
                start[1] = cursor[1];
                *args = transform_relative_point(matrix, args[0], args[1]);
            }
            Data::LineTo(args) | Data::SmoothQuadraticBezierTo(args) => {
                cursor[0] = args[0];
                cursor[1] = args[1];
                *args = transform_absolute_point(matrix, args[0], args[1]);
            }
            Data::LineBy(args) | Data::SmoothQuadraticBezierBy(args) => {
                cursor[0] += args[0];
                cursor[1] += args[1];
                *args = transform_relative_point(matrix, args[0], args[1]);
            }
            Data::CubicBezierTo(args) => {
                cursor[0] = args[4];
                cursor[1] = args[5];
                let p1 = transform_absolute_point(matrix, args[0], args[1]);
                let p2 = transform_absolute_point(matrix, args[2], args[3]);
                let p = transform_absolute_point(matrix, args[4], args[5]);
                *args = [p1[0], p1[1], p2[0], p2[1], p[0], p[1]];
            }
            Data::CubicBezierBy(args) => {
                cursor[0] += args[4];
                cursor[1] += args[5];
                let p1 = transform_relative_point(matrix, args[0], args[1]);
                let p2 = transform_relative_point(matrix, args[2], args[3]);
                let p = transform_relative_point(matrix, args[4], args[5]);
                *args = [p1[0], p1[1], p2[0], p2[1], p[0], p[1]];
            }
            Data::SmoothBezierTo(args) | Data::QuadraticBezierTo(args) => {
                cursor[0] = args[2];
                cursor[1] = args[3];
                let p1 = transform_absolute_point(matrix, args[0], args[1]);
                let p = transform_absolute_point(matrix, args[2], args[3]);
                *args = [p1[0], p1[1], p[0], p[1]];
            }
            Data::SmoothBezierBy(args) | Data::QuadraticBezierBy(args) => {
                cursor[0] += args[2];
                cursor[1] += args[3];
                let p1 = transform_relative_point(matrix, args[0], args[1]);
                let p = transform_relative_point(matrix, args[2], args[3]);
                *args = [p1[0], p1[1], p[0], p[1]];
            }
            Data::ArcTo(args) => {
                transform_arc(cursor, args, matrix);
                cursor[0] = args[5];
                cursor[1] = args[6];
                if f64::abs(args[2]) > 80.0 {
                    args.swap(0, 1);
                    args[2] += if args[2] > 0.0 { -90.0 } else { 90.0 };
                }
                let p = transform_absolute_point(matrix, args[5], args[6]);
                args[5] = p[0];
                args[6] = p[1];
            }
            Data::ArcBy(args) => {
                transform_arc([0.0; 2], args, matrix);
                cursor[0] += args[5];
                cursor[1] += args[6];
                if f64::abs(args[2]) > 80.0 {
                    args.swap(0, 1);
                    args[2] += if args[2] > 0.0 { -90.0 } else { 90.0 };
                }
                let p = transform_relative_point(matrix, args[5], args[6]);
                args[5] = p[0];
                args[6] = p[1];
            }
            Data::ClosePath => {
                cursor[0] = start[0];
                cursor[1] = start[1];
            }
            Data::HorizontalLineBy(_)
            | Data::HorizontalLineTo(_)
            | Data::VerticalLineBy(_)
            | Data::VerticalLineTo(_)
            | Data::Implicit(_) => {
                unreachable!("Reached destroyed command type")
            }
        }
    });
}

fn transform_absolute_point(matrix: &[f64; 6], x: f64, y: f64) -> [f64; 2] {
    [
        matrix[0] * x + matrix[2] * y + matrix[4],
        matrix[1] * x + matrix[3] * y + matrix[5],
    ]
}

fn transform_relative_point(matrix: &[f64; 6], x: f64, y: f64) -> [f64; 2] {
    [matrix[0] * x + matrix[2] * y, matrix[1] * x + matrix[3] * y]
}

fn transform_arc(cursor: [f64; 2], args: &mut [f64; 7], matrix: &[f64; 6]) {
    let x = args[5] - cursor[0];
    let y = args[6] - cursor[1];
    let [a, b, cos, sin] = rotated_ellipse(args, [x, y]);

    let ellipse = [a * cos, a * sin, -b * sin, b * cos, 0.0, 0.0];
    let new_matrix = multiply_transform_matrices(matrix, ellipse);
    let last_col = new_matrix[2] * new_matrix[2] + new_matrix[3] * new_matrix[3];
    let square_sum = new_matrix[0] * new_matrix[0] + new_matrix[1] * new_matrix[1] + last_col;
    let root = f64::hypot(new_matrix[0] - new_matrix[3], new_matrix[1] + new_matrix[2])
        * f64::hypot(new_matrix[0] + new_matrix[3], new_matrix[1] - new_matrix[2]);

    if root == 0.0 {
        args[0] = f64::sqrt(square_sum / 2.0);
        args[1] = args[0];
        args[2] = 0.0;
    } else {
        let major_axis_square = (square_sum + root) / 2.0;
        let minor_axis_square = (square_sum - root) / 2.0;
        let major = f64::abs(major_axis_square - last_col) > 1e-6;
        let sub = if major {
            major_axis_square
        } else {
            minor_axis_square
        } - last_col;
        let rows_sum = new_matrix[0] * new_matrix[2] + new_matrix[1] * new_matrix[3];
        let term_1 = new_matrix[0] * sub + new_matrix[2] * rows_sum;
        let term_2 = new_matrix[1] * sub + new_matrix[3] * rows_sum;
        let term = if major { term_1 } else { term_2 };
        args[0] = major_axis_square.sqrt();
        args[1] = minor_axis_square.sqrt();
        let term_sign = if (major && term_2 < 0.0) || (!major && term_1 > 0.0) {
            -1.0
        } else {
            1.0
        };
        args[2] = (term_sign * f64::acos(term / f64::hypot(term_1, term_2)) * 180.0)
            / std::f64::consts::PI;
    }

    if (matrix[0] < 0.0) != (matrix[3] < 0.0) {
        args[4] = 1.0 - args[4];
    }
}

fn rotated_ellipse(args: &mut [f64; 7], point: [f64; 2]) -> [f64; 4] {
    let rotation = (args[2] * std::f64::consts::PI) / 180.0;
    let cos = f64::cos(rotation);
    let sin = f64::sin(rotation);

    let mut a = args[0];
    let mut b = args[1];
    if a > 0.0 && b > 0.0 {
        let h = (point[0] * cos + point[1] * sin).powi(2) / (4.0 * a * a)
            + (point[1] * cos - point[0] * sin).powi(2) / (4.0 * b * b);
        if h > 1.0 {
            let h = h.sqrt();
            a *= h;
            b *= h;
        }
    }
    [a, b, cos, sin]
}

fn multiply_transform_matrices(matrix: &[f64; 6], ellipse: [f64; 6]) -> [f64; 6] {
    [
        matrix[0] * ellipse[0] + matrix[2] * ellipse[1],
        matrix[1] * ellipse[0] + matrix[3] * ellipse[1],
        matrix[0] * ellipse[2] + matrix[2] * ellipse[3],
        matrix[1] * ellipse[2] + matrix[3] * ellipse[3],
        matrix[0] * ellipse[4] + matrix[2] * ellipse[5] + matrix[4],
        matrix[1] * ellipse[4] + matrix[3] * ellipse[5] + matrix[5],
    ]
}

lazy_static! {
    static ref TRANSFORM_ID: style::Id<'static> = style::Id::Attr(PresentationAttrId::Transform);
}

#[test]
#[allow(clippy::too_many_lines)]
fn apply_transforms() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <path transform="translate(100,0)" d="M0,0 V100 L 70,50 z M70,50 L140,0 V100 z"/>
    <path transform="" d="M0,0 V100 L 70,50 z M70,50 L140,0 V100 z"/>
    <path fill="red" transform="rotate(15) scale(.5) skewX(5) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    <path fill="red" stroke="red" transform="rotate(15) scale(.5) skewX(5) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    <path fill="red" stroke="red" transform="rotate(15) scale(.5) skewX(5) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 a150,150 0 1,0 150,-150 z"/>
    <path fill="red" stroke="red" transform="rotate(15) scale(.5) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    <path fill="red" stroke="red" transform="rotate(15) scale(1.5) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    <path fill="red" stroke="red" transform="rotate(15) scale(0.33) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    <g stroke="red">
        <path fill="red" transform="rotate(15) scale(.5) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    </g>
    <g stroke="red" stroke-width="2">
        <path fill="red" transform="rotate(15) scale(.5) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    </g>
    <path transform="scale(10)" id="a" d="M0,0 V100 L 70,50 z M70,50 L140,0 V100 z"/>
    <path transform="scale(10)" id="a" d="M0,0 V100 L 70,50 z M70,50 L140,0 V100 z" stroke="#000"/>
    <path transform="scale(10)" id="a" d="M0,0 V100 L 70,50 z M70,50 L140,0 V100 z" stroke="#000" stroke-width=".5"/>
    <g stroke="#000" stroke-width="5">
        <path transform="scale(10)" id="a" d="M0,0 V100 L 70,50 z M70,50 L140,0 V100 z"/>
    </g>
    <path fill="url(#gradient)" transform="rotate(15) scale(0.33) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    <path clip-path="url(#a)" transform="rotate(15) scale(0.33) translate(200,100)" d="M100,200 300,400 H100 V300 C100,100 250,100 250,200 S400,300 400,200 Q400,50 600,300 T1000,300 z"/>
    <path d="M5 0a10 10 0 1 0 20 0" transform="matrix(1 0 0 1 5 0)"/>
    <path d="M5 0a10 10 0 1 0 20 0" transform="rotate(15) scale(.8,1.2) "/>
    <path d="M5 0a10 10 0 1 0 20 0" transform="rotate(45)"/>
    <path d="M5 0a10 10 0 1 0 20 0" transform="skewX(45)"/>
    <path d="M0 300a1 2 0 1 0 200 0a1 2 0 1 0 -200 0" transform="rotate(15 100 300) scale(.8 1.2)"/>
    <path d="M0 300a1 2 0 1 0 200 0a1 2 0 1 0 -200 0" transform="rotate(15 100 300)"/>
    <path d="M700 300a1 2 0 1 0 200 0a1 2 0 1 0 -200 0" transform="rotate(-75 700 300) scale(.8 1.2)"/>
    <path d="M12.6 8.6l-3.1-3.2-3.1 3.2-.8-.7 3.9-3.9 3.9 3.9zM9 5h1v10h-1z" transform="rotate(-90 9.5 9.5)"/>
    <path d="M637.43 482.753a43.516 94.083 0 1 1-87.033 0 43.516 94.083 0 1 1 87.032 0z" transform="matrix(1.081 .234 -.187 .993 -37.573 -235.766)"/>
    <path d="m-1.26-1.4a6.53 1.8-15.2 1 1 12.55-3.44" transform="translate(0, 0)"/>
    <path d="M0 0c.07 1.33.14 2.66.21 3.99.07 1.33.14 2.66.21 3.99"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 36">
    <path d="M32 4a4 4 0 0 0-4-4H8a4 4 0 0 1-4 4v28a4 4 0 0 1 4 4h20a4 4 0 0 0 4-4V4z" fill="#888" transform="matrix(1 0 0 -1 0 36)"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 500">
    <path transform="translate(250, 250) scale(1.5, 1.5) translate(-250, -250)" fill="#7ED321" stroke="#000" stroke-width="15" vector-effect="non-scaling-stroke" d="M125 125h250v250h-250v-250z"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r#"<svg width="480" height="360" xmlns="http://www.w3.org/2000/svg">
  <path transform="scale(1.8)" stroke="black" stroke-width="10" fill="none" stroke-dasharray="none" d="   M  20 20   L  200 20"/>
  <path transform="scale(1.8)" stroke="black" stroke-width="10" fill="none" stroke-dasharray="0" d="   M  20 40   L  200 40"/>
  <path transform="scale(1.8)" stroke="black" stroke-width="20" fill="none" stroke-dasharray="5,2,5,5,2,5" d="   M  20 60   L  200 60"/>
  <path transform="scale(1.8)" stroke="blue" stroke-width="10" fill="none" stroke-dasharray="5,2,5" d="   M  20 60   L  200 60"/>
  <path transform="scale(1.8)" stroke="black" stroke-width="10" fill="none" stroke-dasharray="2" d="   M  20 80   L  200 80"/>
  <path transform="scale(1.8)" stroke="blue" stroke-width="10" fill="none" stroke-dasharray="2" stroke-dashoffset="2" d="         M  20 90   L  200 90"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r#"<svg width="200" height="100">
  <path transform="scale(2)" style="stroke:black;stroke-width:10;" d="M 20 20 H 80" />
  <path transform="scale(2)" stroke="black" stroke-width="10" d="M 20 20 H 80" />
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r#"<svg width="1200" height="1200">
  <path transform="translate(100) scale(2)" d="m200 200 h-100 a100 100 0 1 0 100 -100 z"/>
  <path transform="translate(100) scale(2)" d="M400 200 H300 A100 100 0 1 0 400 100 z"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 400" fill="#E7DACB">
  <path
    d="
      M 152 65
      V 158
      H 49
      V 65
      z
      m -14 75
      V 83
      H 67
      V 141
      z
    "
    transform="translate(-24, -41)"
  />
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "applyTransforms": {}, "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 31.6 31.6">
  <path d="m5.25,2.2H25.13a0,0,0,0,1-.05-.05V14.18Z" transform="translate(0 0)"/>
</svg>"#
        )
    )?);

    Ok(())
}
