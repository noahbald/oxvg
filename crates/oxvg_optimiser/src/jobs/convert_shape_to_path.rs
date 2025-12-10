use std::cell;

use lightningcss::{selector::Component, visit_types, visitor::Visit};
use oxvg_ast::{
    element::Element,
    get_attribute, has_attribute, remove_attribute, set_attribute,
    visitor::{Context, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    attribute::{path, presentation::LengthPercentage, uncategorised::Radius, AttrId},
    element::ElementId,
};
use oxvg_path::{command::Data, convert, Path};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::JobsError;

use super::convert_path_data::ConvertPrecision;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// Converts basic shapes to `<path>` elements
///
/// # Differences to SVGO
///
/// OXVG will avoid converting shapes which may be referenced by local-name
/// in stylesheets.
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
    #[cfg_attr(feature = "serde", serde(default = "default_convert_arcs"))]
    pub convert_arcs: bool,
    /// The number of decimal places to round to
    #[cfg_attr(feature = "wasm", tsify(type = "null | false | number"))]
    #[cfg_attr(feature = "serde", serde(default = "ConvertPrecision::default"))]
    pub float_precision: ConvertPrecision,
}

impl Default for ConvertShapeToPath {
    fn default() -> Self {
        Self {
            convert_arcs: default_convert_arcs(),
            float_precision: ConvertPrecision::default(),
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for ConvertShapeToPath {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<oxvg_ast::visitor::PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        let mut state = State {
            options: self,
            referenced_shapes: ReferencedShapes::empty(),
        };
        for styles in &context.query_has_stylesheet_result {
            styles.borrow_mut().0.visit(&mut state)?;
        }
        if !state.referenced_shapes.contains(ReferencedShapes::Path) {
            state.start_with_context(document, context)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

struct State<'o> {
    options: &'o ConvertShapeToPath,
    referenced_shapes: ReferencedShapes,
}

bitflags! {
    #[derive(Debug)]
    pub struct ReferencedShapes: usize {
        const Rect = 1 << 0;
        const Line = 1 << 1;
        const Polyline = 1 << 2;
        const Polygon = 1 << 3;
        const Circle = 1 << 4;
        const Ellipse = 1 << 5;
        const Path = 1 << 6;
    }
}

impl<'input> lightningcss::visitor::Visitor<'input> for State<'_> {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(SELECTORS)
    }

    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'input>,
    ) -> Result<(), Self::Error> {
        let mut iter = selector.iter();
        loop {
            for token in &mut iter {
                let Component::LocalName(name) = token else {
                    continue;
                };
                match &*name.lower_name.0 {
                    "rect" => self.referenced_shapes.insert(ReferencedShapes::Rect),
                    "line" => self.referenced_shapes.insert(ReferencedShapes::Line),
                    "polyline" => self.referenced_shapes.insert(ReferencedShapes::Polyline),
                    "polygon" => self.referenced_shapes.insert(ReferencedShapes::Polygon),
                    "circle" => self.referenced_shapes.insert(ReferencedShapes::Circle),
                    "ellipse" => self.referenced_shapes.insert(ReferencedShapes::Ellipse),
                    "path" => self.referenced_shapes.insert(ReferencedShapes::Path),
                    _ => {}
                }
            }
            if iter.next_sequence().is_none() {
                break;
            }
        }
        Ok(())
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'_> {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let name = element.qual_name();

        let options = &self.options;
        let path_options = &convert::Options {
            precision: options.float_precision.0,
            ..convert::Options::default()
        };
        let convert_arcs = options.convert_arcs;

        match name {
            ElementId::Rect if !self.referenced_shapes.contains(ReferencedShapes::Rect) => {
                ConvertShapeToPath::rect_to_path(element, path_options, context.info);
            }
            ElementId::Line if !self.referenced_shapes.contains(ReferencedShapes::Line) => {
                ConvertShapeToPath::line_to_path(element, path_options, context.info);
            }
            ElementId::Polyline if !self.referenced_shapes.contains(ReferencedShapes::Polyline) => {
                ConvertShapeToPath::poly_to_path(element, path_options, false, context.info);
            }
            ElementId::Polygon if !self.referenced_shapes.contains(ReferencedShapes::Polygon) => {
                ConvertShapeToPath::poly_to_path(element, path_options, true, context.info);
            }
            ElementId::Circle
                if convert_arcs && !self.referenced_shapes.contains(ReferencedShapes::Circle) =>
            {
                ConvertShapeToPath::circle_to_path(element, path_options, context.info);
            }

            ElementId::Ellipse
                if convert_arcs && !self.referenced_shapes.contains(ReferencedShapes::Circle) =>
            {
                ConvertShapeToPath::ellipse_to_path(element, path_options, context.info);
            }

            _ => {}
        }
        Ok(())
    }
}

#[expect(clippy::needless_pass_by_value)]
fn lp_px(lp: cell::Ref<LengthPercentage>) -> Option<f64> {
    use lightningcss::values::length::LengthPercentage;
    match &lp.0 {
        LengthPercentage::Dimension(d) => d.to_px().map(|px| px as f64),
        _ => None,
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

        let Some(x) = (match get_attribute!(element, XGeometry) {
            Some(x) => lp_px(x),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(y) = (match get_attribute!(element, YGeometry) {
            Some(y) => lp_px(y),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(width) = get_attribute!(element, WidthRect).and_then(lp_px) else {
            return;
        };
        let Some(height) = get_attribute!(element, HeightRect).and_then(lp_px) else {
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

        set_attribute!(element, D(path::Path(path, None)));
        element.remove_attribute(&AttrId::XGeometry);
        element.remove_attribute(&AttrId::YGeometry);
        element.remove_attribute(&AttrId::WidthRect);
        element.remove_attribute(&AttrId::HeightRect);
        let _ = element.set_local_name(ElementId::Path, &info.allocator);
    }

    fn line_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        info: &Info<'input, 'arena>,
    ) {
        let Some(x1) = (match get_attribute!(element, X1Line) {
            Some(x1) => lp_px(x1),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(y1) = (match get_attribute!(element, Y1Line) {
            Some(y1) => lp_px(y1),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(x2) = (match get_attribute!(element, X2Line) {
            Some(x2) => lp_px(x2),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(y2) = (match get_attribute!(element, Y2Line) {
            Some(y2) => lp_px(y2),
            None => Some(0.0),
        }) else {
            return;
        };

        let mut path = Path(vec![
            Data::MoveTo([x1, y1]),
            Data::Implicit(Box::new(Data::LineTo([x2, y2]))),
        ]);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path, None)));
        element.remove_attribute(&AttrId::X1Line);
        element.remove_attribute(&AttrId::Y1Line);
        element.remove_attribute(&AttrId::X2Line);
        element.remove_attribute(&AttrId::Y2Line);
        let _ = element.set_local_name(ElementId::Path, &info.allocator);
    }

    fn poly_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        is_polygon: bool,
        info: &Info<'input, 'arena>,
    ) {
        let Some(points) = remove_attribute!(element, Points) else {
            // Remove element with invalid or missing points
            element.remove();
            return;
        };
        let mut data = points.0 .0;
        if data.len() <= 1 {
            // Remove pointless data ;)
            element.remove();
            return;
        }
        if is_polygon {
            data.push(Data::ClosePath);
        }
        let mut path = Path(data);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path, None)));
        let _ = element.set_local_name(ElementId::Path, &info.allocator);
    }

    #[allow(clippy::similar_names)]
    fn circle_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        info: &Info<'input, 'arena>,
    ) {
        let Some(cx) = (match get_attribute!(element, CXGeometry) {
            Some(cx) => lp_px(cx),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(cy) = (match get_attribute!(element, CYGeometry) {
            Some(cy) => lp_px(cy),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(r) = (match get_attribute!(element, RGeometry) {
            Some(r) => lp_px(r),
            None => Some(0.0),
        }) else {
            return;
        };

        let mut path = Path(vec![
            Data::MoveTo([cx, cy - r]),
            Data::ArcTo([r, r, 0.0, 1.0, 0.0, cx, cy + r]),
            Data::Implicit(Box::new(Data::ArcTo([r, r, 0.0, 1.0, 0.0, cx, cy - r]))),
            Data::ClosePath,
        ]);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path, None)));
        element.remove_attribute(&AttrId::CXGeometry);
        element.remove_attribute(&AttrId::CYGeometry);
        element.remove_attribute(&AttrId::RGeometry);
        let _ = element.set_local_name(ElementId::Path, &info.allocator);
    }

    #[allow(clippy::similar_names)]
    fn ellipse_to_path<'input, 'arena>(
        element: &Element<'input, 'arena>,
        options: &convert::Options,
        info: &Info<'input, 'arena>,
    ) {
        let Some(cx) = (match get_attribute!(element, CXGeometry) {
            Some(cx) => lp_px(cx),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(cy) = (match get_attribute!(element, CYGeometry) {
            Some(cy) => lp_px(cy),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(rx) = (match get_attribute!(element, RX) {
            Some(rx) => r_px(rx),
            None => Some(0.0),
        }) else {
            return;
        };
        let Some(ry) = (match get_attribute!(element, RY) {
            Some(ry) => r_px(ry),
            None => Some(0.0),
        }) else {
            return;
        };

        let mut path = Path(vec![
            Data::MoveTo([cx, cy - ry]),
            Data::ArcTo([rx, ry, 0.0, 1.0, 0.0, cx, cy + ry]),
            Data::Implicit(Box::new(Data::ArcTo([rx, ry, 0.0, 1.0, 0.0, cx, cy - ry]))),
            Data::ClosePath,
        ]);
        options.round_path(&mut path, options.error());

        set_attribute!(element, D(path::Path(path, None)));
        element.remove_attribute(&AttrId::CXGeometry);
        element.remove_attribute(&AttrId::CYGeometry);
        element.remove_attribute(&AttrId::RX);
        element.remove_attribute(&AttrId::RY);
        let _ = element.set_local_name(ElementId::Path, &info.allocator);
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
