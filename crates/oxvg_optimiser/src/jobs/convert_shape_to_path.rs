use std::cell;

use itertools::peek_nth;
use lightningcss::values::length::{Length, LengthOrNumber};
use oxvg_ast::{
    attribute::data::{path, presentation::LengthPercentage, uncategorised::Radius, AttrId},
    element::{data::ElementId, Element},
    get_attribute, has_attribute, set_attribute,
    visitor::{Context, Info, Visitor},
};
use oxvg_path::{command::Data, convert, Path};
use serde::{Deserialize, Serialize};

use crate::error::JobsError;

use super::convert_path_data::ConvertPrecision;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
/// Converts basic shapes to `<path>` elements
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
pub struct ConvertShapeToPath {
    /// Whether to convert `<circle>` and `<ellipses>` to paths.
    #[serde(default = "default_convert_arcs")]
    pub convert_arcs: bool,
    /// The number of decimal places to round to
    #[cfg_attr(feature = "wasm", tsify(type = "null | false | number"))]
    #[serde(default = "ConvertPrecision::default")]
    pub float_precision: ConvertPrecision,
}

impl<'input, 'arena> Visitor<'input, 'arena> for ConvertShapeToPath {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let name = element.qual_name();

        let path_options = &convert::Options {
            precision: self.float_precision.0,
            ..convert::Options::default()
        };
        let convert_arcs = self.convert_arcs;

        match name {
            ElementId::Rect => Self::rect_to_path(element, path_options, context.info),
            ElementId::Line => Self::line_to_path(element, path_options, context.info),
            ElementId::Polyline => Self::poly_to_path(element, path_options, false, context.info),
            ElementId::Polygon => Self::poly_to_path(element, path_options, true, context.info),
            ElementId::Circle if convert_arcs => {
                Self::circle_to_path(element, path_options, context.info)
            }
            ElementId::Ellipse if convert_arcs => {
                Self::ellipse_to_path(element, path_options, context.info)
            }
            _ => {}
        }
        Ok(())
    }
}

fn lp_px(lp: cell::Ref<LengthPercentage>) -> Option<f64> {
    use lightningcss::values::length::LengthPercentage;
    match &lp.0 {
        LengthPercentage::Dimension(d) => d.to_px().map(|px| px as f64),
        _ => None,
    }
}
fn ln_px(ln: cell::Ref<LengthOrNumber>) -> Option<f64> {
    match &*ln {
        LengthOrNumber::Number(n) => Some(*n as f64),
        LengthOrNumber::Length(Length::Value(l)) => l.to_px().map(|px| px as f64),
        LengthOrNumber::Length(Length::Calc(_)) => None,
    }
}
fn r_px(r: cell::Ref<Radius>) -> Option<f64> {
    cell::Ref::filter_map(r, |r| match r {
        Radius::LengthPercentage(lp) => Some(lp),
        Radius::Auto => None,
    })
    .ok()
    .and_then(lp_px)
}
impl ConvertShapeToPath {
    fn rect_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        info: &Info<'input, 'arena>,
    ) {
        if has_attribute!(element, RX | RY) {
            return;
        }

        let Some(x) = get_attribute!(element, X).and_then(lp_px) else {
            return;
        };
        let Some(y) = get_attribute!(element, Y).and_then(lp_px) else {
            return;
        };
        let Some(width) = get_attribute!(element, Width).and_then(lp_px) else {
            return;
        };
        let Some(height) = get_attribute!(element, Height).and_then(lp_px) else {
            return;
        };

        let mut path = Path(vec![
            Data::MoveTo([x, y]),
            Data::HorizontalLineTo([x + width]),
            Data::VerticalLineTo([y + height]),
            Data::HorizontalLineTo([x]),
            Data::ClosePath,
        ]);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path)));
        element.remove_attribute(&AttrId::X);
        element.remove_attribute(&AttrId::Y);
        element.remove_attribute(&AttrId::Width);
        element.remove_attribute(&AttrId::Height);
        element.set_local_name(ElementId::Path, &info.allocator);
    }

    fn line_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        info: &Info<'input, 'arena>,
    ) {
        let Some(x1) = get_attribute!(element, X1).and_then(ln_px) else {
            return;
        };
        let Some(y1) = get_attribute!(element, Y1).and_then(ln_px) else {
            return;
        };
        let Some(x2) = get_attribute!(element, X2).and_then(ln_px) else {
            return;
        };
        let Some(y2) = get_attribute!(element, Y2).and_then(ln_px) else {
            return;
        };

        let mut path = Path(vec![
            Data::MoveTo([x1, y1]),
            Data::Implicit(Box::new(Data::LineTo([x2, y2]))),
        ]);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path)));
        element.remove_attribute(&AttrId::X1);
        element.remove_attribute(&AttrId::Y1);
        element.remove_attribute(&AttrId::X2);
        element.remove_attribute(&AttrId::Y2);
        element.set_local_name(ElementId::Path, &info.allocator);
    }

    fn poly_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        is_polygon: bool,
        info: &Info<'input, 'arena>,
    ) {
        let Some(coords) = get_attribute!(element, Points) else {
            return;
        };
        let mut coords = peek_nth(coords.list.iter());

        if coords.peek_nth(3).is_none() {
            element.remove();
            return;
        }

        let mut data = Vec::with_capacity(2);
        while let (Some(x), Some(y)) = (coords.next(), coords.next()) {
            let x = *x as f64;
            let y = *y as f64;
            let command = if data.is_empty() {
                Data::MoveTo([x, y])
            } else {
                Data::Implicit(Box::new(Data::LineTo([x, y])))
            };
            data.push(command);
        }
        if is_polygon {
            data.push(Data::ClosePath);
        }
        let mut path = Path(data);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path)));
        element.remove_attribute(&AttrId::Points);
        element.set_local_name(ElementId::Path, &info.allocator);
    }

    #[allow(clippy::similar_names)]
    fn circle_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        info: &Info<'input, 'arena>,
    ) {
        let Some(cx) = get_attribute!(element, CX).and_then(lp_px) else {
            return;
        };
        let Some(cy) = get_attribute!(element, CY).and_then(lp_px) else {
            return;
        };
        let Some(r) = get_attribute!(element, RCircle).and_then(lp_px) else {
            return;
        };

        let mut path = Path(vec![
            Data::MoveTo([cx, cy - r]),
            Data::ArcTo([r, r, 0.0, 1.0, 0.0, cx, cy + r]),
            Data::Implicit(Box::new(Data::ArcTo([r, r, 0.0, 1.0, 0.0, cx, cy - r]))),
            Data::ClosePath,
        ]);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path)));
        element.remove_attribute(&AttrId::CX);
        element.remove_attribute(&AttrId::CY);
        element.remove_attribute(&AttrId::RCircle);
        element.set_local_name(ElementId::Path, &info.allocator);
    }

    #[allow(clippy::similar_names)]
    fn ellipse_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        info: &Info<'input, 'arena>,
    ) {
        let Some(cx) = get_attribute!(element, CX).and_then(lp_px) else {
            return;
        };
        let Some(cy) = get_attribute!(element, CY).and_then(lp_px) else {
            return;
        };
        let Some(rx) = get_attribute!(element, RX).and_then(r_px) else {
            return;
        };
        let Some(ry) = get_attribute!(element, RY).and_then(r_px) else {
            return;
        };

        let mut path = Path(vec![
            Data::MoveTo([cx, cy - ry]),
            Data::ArcTo([rx, ry, 0.0, 1.0, 0.0, cx, cy + ry]),
            Data::Implicit(Box::new(Data::ArcTo([rx, ry, 0.0, 1.0, 0.0, cx, cy - ry]))),
            Data::ClosePath,
        ]);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path)));
        element.remove_attribute(&AttrId::CX);
        element.remove_attribute(&AttrId::CY);
        element.remove_attribute(&AttrId::RX);
        element.remove_attribute(&AttrId::RY);
        element.set_local_name(ElementId::Path, &info.allocator);
    }
}

const fn default_convert_arcs() -> bool {
    false
}

#[test]
fn convert_shape_to_path() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertShapeToPath": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <rect width="100%"/>
    <rect width="100%" height="100%"/>
    <rect x="25%" y="25%" width="50%" height="50%"/>
    <rect x="25pt" y="25pt" width="50pt" height="50pt"/>
    <rect x="10" y="10" width="50" height="50" rx="4"/>
    <rect x="0" y="0" width="20" height="20" ry="5"/>
    <rect width="32" height="32"/>
    <rect x="20" y="10" width="50" height="40"/>
    <rect fill="#666" x="10" y="10" width="10" height="10"/>
</svg>
"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertShapeToPath": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <line x2="100%" y2="100%"/>
    <line x1="24" y2="24"/>
    <line x1="10" y1="10" x2="50" y2="20"/>
    <line stroke="#000" x1="10" y1="10" x2="50" y2="20"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertShapeToPath": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <polyline points="10,10 20"/>
    <polyline points="10,80 20,50 50,20 80,10"/>
    <polyline points="20 ,10  50    40 30.5-1e-1 , 20 10"/>
    <polyline stroke="#000" points="10,10 20,20 10,20"/>
    <polygon points="10,10 20"/>
    <polygon points="10,80 20,50 50,20 80,10"/>
    <polygon points="20 10  50 40 30,20"/>
    <polygon stroke="#000" points="10,10 20,20 10,20"/>
    <polygon stroke="none" points="10,10 20,20 10,20"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertShapeToPath": { "convertArcs": true } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <circle cx="10" cy="10" r="5"/>
    <ellipse cx="10" cy="10" rx="5" ry="5"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertShapeToPath": { "convertArcs": true, "floatPrecision": 3 } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="65mm" height="45mm" viewBox="0 0 65 45">
  <rect x="26.614" y="29.232" width="34.268" height="8.1757"/>
  <line x1="26.6142" y1="29.2322" x2="34.2682" y2="8.1757"/>
  <polyline points="26.6142,29.2322 34.2682,8.1757"/>
  <polygon points="26.6142,29.2322 34.2682,8.1757"/>
  <circle cx="26.6142" cy="29.2322" r="34.2682"/>
  <ellipse cx="26.6142" cy="29.2322" rx="34.2682" ry="8.1757"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertShapeToPath": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
  <defs>
    <rect id="rect1" width="120" height="120" />
  </defs>
</svg>"#
        ),
    )?);

    Ok(())
}
