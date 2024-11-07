use std::{f64, rc::Rc};

use itertools::peek_nth;
use markup5ever::local_name;
use oxvg_path::{command::Data, convert, Path};
use oxvg_selectors::Element;
use serde::Deserialize;

use crate::{Context, Job};

use super::convert_path_data::Precision;

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertShapeToPath {
    convert_arcs: Option<bool>,
    float_precision: Option<Precision>,
}

impl Job for ConvertShapeToPath {
    fn run(&self, node: &Rc<rcdom::Node>, _context: &Context) {
        let element = &mut Element::new(node.clone());
        let Some(name) = element.get_name() else {
            return;
        };

        let path_options = &convert::Options {
            precision: self.float_precision.unwrap_or_default().0,
            ..convert::Options::default()
        };
        let convert_arcs = self.convert_arcs.unwrap_or(false);

        match name {
            local_name!("rect") => Self::rect_to_path(element, path_options),
            local_name!("line") => Self::line_to_path(element, path_options),
            local_name!("polyline") => Self::poly_to_path(element, path_options, false),
            local_name!("polygon") => Self::poly_to_path(element, path_options, true),
            local_name!("circle") if convert_arcs => Self::circle_to_path(element, path_options),
            local_name!("ellipse") if convert_arcs => Self::ellipse_to_path(element, path_options),
            _ => {}
        }
    }
}

impl ConvertShapeToPath {
    fn rect_to_path(element: &mut Element, options: &convert::Options) {
        if element.get_attr(&local_name!("rx")).is_some()
            || element.get_attr(&local_name!("ry")).is_some()
        {
            return;
        }

        let x = element
            .get_attr(&local_name!("x"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let y = element
            .get_attr(&local_name!("y"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let Some(width) = element
            .get_attr(&local_name!("width"))
            .map(|a| String::from(a.value))
        else {
            return;
        };
        let Some(height) = element
            .get_attr(&local_name!("height"))
            .map(|a| String::from(a.value))
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

        element.set_attr(&local_name!("d"), path.to_string().into());
        element.remove_attr(&local_name!("x"));
        element.remove_attr(&local_name!("y"));
        element.remove_attr(&local_name!("width"));
        element.remove_attr(&local_name!("height"));
        element.set_name(local_name!("path"));
    }

    fn line_to_path(element: &mut Element, options: &convert::Options) {
        let x1 = element
            .get_attr(&local_name!("x1"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let y1 = element
            .get_attr(&local_name!("y1"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let x2 = element
            .get_attr(&local_name!("x2"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let y2 = element
            .get_attr(&local_name!("y2"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));

        let Ok(x1) = x1.parse::<f64>() else { return };
        let Ok(y1) = y1.parse::<f64>() else { return };
        let Ok(x2) = x2.parse::<f64>() else { return };
        let Ok(y2) = y2.parse::<f64>() else { return };

        let mut path = Path(vec![
            Data::MoveTo([x1, y1]),
            Data::Implicit(Box::new(Data::LineTo([x2, y2]))),
        ]);
        options.round_path(&mut path, options.error());

        element.set_attr(&local_name!("d"), path.to_string().into());
        element.remove_attr(&local_name!("x1"));
        element.remove_attr(&local_name!("y1"));
        element.remove_attr(&local_name!("x2"));
        element.remove_attr(&local_name!("y2"));
        element.set_name(local_name!("path"));
    }

    fn poly_to_path(element: &mut Element, options: &convert::Options, is_polygon: bool) {
        let Some(coords) = element
            .get_attr(&local_name!("points"))
            .map(|a| String::from(a.value))
        else {
            return;
        };

        let coords = oxvg_selectors::regex::NUMERIC_VALUES
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

        element.set_attr(&local_name!("d"), path.to_string().into());
        element.remove_attr(&local_name!("points"));
        element.set_name(local_name!("path"));
    }

    fn circle_to_path(element: &mut Element, options: &convert::Options) {
        let cx = element
            .get_attr(&local_name!("cx"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let cy = element
            .get_attr(&local_name!("cy"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let r = element
            .get_attr(&local_name!("r"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));

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

        element.set_attr(&local_name!("d"), path.to_string().into());
        element.remove_attr(&local_name!("cx"));
        element.remove_attr(&local_name!("cy"));
        element.remove_attr(&local_name!("r"));
        element.set_name(local_name!("path"));
    }

    fn ellipse_to_path(element: &mut Element, options: &convert::Options) {
        let cx = element
            .get_attr(&local_name!("cx"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let cy = element
            .get_attr(&local_name!("cy"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let rx = element
            .get_attr(&local_name!("rx"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));
        let ry = element
            .get_attr(&local_name!("ry"))
            .map_or_else(|| String::from("0"), |a| String::from(a.value));

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

        element.set_attr(&local_name!("d"), path.to_string().into());
        element.remove_attr(&local_name!("cx"));
        element.remove_attr(&local_name!("cy"));
        element.remove_attr(&local_name!("rx"));
        element.remove_attr(&local_name!("ry"));
        element.set_name(local_name!("path"));
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
