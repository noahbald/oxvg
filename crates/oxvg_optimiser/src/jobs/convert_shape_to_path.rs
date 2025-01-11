use std::f64;

use itertools::peek_nth;
use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use oxvg_path::{command::Data, convert, Path};
use serde::Deserialize;

use super::convert_path_data::Precision;

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertShapeToPath {
    convert_arcs: Option<bool>,
    float_precision: Option<Precision>,
}

impl<E: Element> Visitor<E> for ConvertShapeToPath {
    type Error = String;

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        let element = &mut element.clone();
        let name = element.local_name();

        let path_options = &convert::Options {
            precision: self.float_precision.unwrap_or_default().0,
            ..convert::Options::default()
        };
        let convert_arcs = self.convert_arcs.unwrap_or(false);

        match name.as_ref() {
            "rect" => Self::rect_to_path(element, path_options),
            "line" => Self::line_to_path(element, path_options),
            "polyline" => Self::poly_to_path(element, path_options, false),
            "polygon" => Self::poly_to_path(element, path_options, true),
            "circle" if convert_arcs => Self::circle_to_path(element, path_options),
            "ellipse" if convert_arcs => Self::ellipse_to_path(element, path_options),
            _ => {}
        }
        Ok(())
    }
}

impl ConvertShapeToPath {
    fn rect_to_path(element: &mut impl Element, options: &convert::Options) {
        if element.has_attribute_local(&"rx".into()) || element.has_attribute_local(&"ry".into()) {
            return;
        }

        let x_name = &"x".into();
        let y_name = &"y".into();
        let width_name = &"width".into();
        let height_name = &"height".into();
        let x = element
            .get_attribute_local(x_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let y = element
            .get_attribute_local(y_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let Some(width) = element
            .get_attribute_local(width_name)
            .map(|a| a.to_string())
        else {
            return;
        };
        let Some(height) = element
            .get_attribute_local(height_name)
            .map(|a| a.to_string())
        else {
            return;
        };

        // Units should be stripped of `px` after `cleanupNumericValues`
        let Ok(x) = x.parse::<f64>() else { return };
        let Ok(y) = y.parse::<f64>() else { return };
        let Ok(width) = width.parse::<f64>() else {
            return;
        };
        let Ok(height) = height.parse::<f64>() else {
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

        element.set_attribute_local("d".into(), path.to_string().into());
        element.remove_attribute_local(x_name);
        element.remove_attribute_local(y_name);
        element.remove_attribute_local(width_name);
        element.remove_attribute_local(height_name);
        element.set_local_name("path".into());
    }

    fn line_to_path(element: &mut impl Element, options: &convert::Options) {
        let x1_name = &"x1".into();
        let y1_name = &"y1".into();
        let x2_name = &"x2".into();
        let y2_name = &"y2".into();
        let x1 = element
            .get_attribute_local(x1_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let y1 = element
            .get_attribute_local(y1_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let x2 = element
            .get_attribute_local(x2_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let y2 = element
            .get_attribute_local(y2_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());

        let Ok(x1) = x1.parse::<f64>() else { return };
        let Ok(y1) = y1.parse::<f64>() else { return };
        let Ok(x2) = x2.parse::<f64>() else { return };
        let Ok(y2) = y2.parse::<f64>() else { return };

        let mut path = Path(vec![
            Data::MoveTo([x1, y1]),
            Data::Implicit(Box::new(Data::LineTo([x2, y2]))),
        ]);
        options.round_path(&mut path, options.error());

        element.set_attribute_local("d".into(), path.to_string().into());
        element.remove_attribute_local(x1_name);
        element.remove_attribute_local(y1_name);
        element.remove_attribute_local(x2_name);
        element.remove_attribute_local(y2_name);
        element.set_local_name("path".into());
    }

    fn poly_to_path(element: &mut impl Element, options: &convert::Options, is_polygon: bool) {
        let points_name = &"points".into();
        let Some(coords) = element
            .get_attribute_local(points_name)
            .map(|a| a.to_string())
        else {
            return;
        };

        let coords = oxvg_collections::regex::NUMERIC_VALUES
            .find_iter(&coords)
            .map(|item| item.as_str().parse::<f64>());
        let mut coords = peek_nth(coords);

        if coords.peek_nth(3).is_none() {
            element.remove();
            return;
        }

        let mut data = Vec::with_capacity(2);
        while let (Some(x), Some(y)) = (coords.next(), coords.next()) {
            let Ok(x) = x else {
                return;
            };
            let Ok(y) = y else {
                return;
            };

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

        element.set_attribute_local("d".into(), path.to_string().into());
        element.remove_attribute_local(points_name);
        element.set_local_name("path".into());
    }

    #[allow(clippy::similar_names)]
    fn circle_to_path(element: &mut impl Element, options: &convert::Options) {
        let cx_name = &"cx".into();
        let cy_name = &"cy".into();
        let r_name = &"r".into();
        let cx = element
            .get_attribute_local(cx_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let cy = element
            .get_attribute_local(cy_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let r = element
            .get_attribute_local(r_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());

        let Ok(cx) = cx.parse::<f64>() else { return };
        let Ok(cy) = cy.parse::<f64>() else { return };
        let Ok(r) = r.parse::<f64>() else { return };

        let mut path = Path(vec![
            Data::MoveTo([cx, cy - r]),
            Data::ArcTo([r, r, 0.0, 1.0, 0.0, cx, cy + r]),
            Data::Implicit(Box::new(Data::ArcTo([r, r, 0.0, 1.0, 0.0, cx, cy - r]))),
            Data::ClosePath,
        ]);
        options.round_path(&mut path, options.error());

        element.set_attribute_local("d".into(), path.to_string().into());
        element.remove_attribute_local(cx_name);
        element.remove_attribute_local(cy_name);
        element.remove_attribute_local(r_name);
        element.set_local_name("path".into());
    }

    #[allow(clippy::similar_names)]
    fn ellipse_to_path(element: &mut impl Element, options: &convert::Options) {
        let cx_name = &"cx".into();
        let cy_name = &"cy".into();
        let rx_name = &"rx".into();
        let ry_name = &"ry".into();
        let cx = element
            .get_attribute_local(cx_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let cy = element
            .get_attribute_local(cy_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let rx = element
            .get_attribute_local(rx_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());
        let ry = element
            .get_attribute_local(ry_name)
            .map_or_else(|| String::from("0"), |a| a.to_string());

        let Ok(cx) = cx.parse::<f64>() else { return };
        let Ok(cy) = cy.parse::<f64>() else { return };
        let Ok(rx) = rx.parse::<f64>() else { return };
        let Ok(ry) = ry.parse::<f64>() else { return };

        let mut path = Path(vec![
            Data::MoveTo([cx, cy - ry]),
            Data::ArcTo([rx, ry, 0.0, 1.0, 0.0, cx, cy + ry]),
            Data::Implicit(Box::new(Data::ArcTo([rx, ry, 0.0, 1.0, 0.0, cx, cy - ry]))),
            Data::ClosePath,
        ]);
        options.round_path(&mut path, options.error());

        element.set_attribute_local("d".into(), path.to_string().into());
        element.remove_attribute_local(cx_name);
        element.remove_attribute_local(cy_name);
        element.remove_attribute_local(rx_name);
        element.remove_attribute_local(ry_name);
        element.set_local_name("path".into());
    }
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

    Ok(())
}
