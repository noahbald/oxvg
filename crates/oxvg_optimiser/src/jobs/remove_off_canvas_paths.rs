use std::cell::RefCell;

use lightningcss::values::percentage::DimensionPercentage;
use oxvg_ast::{
    element::Element,
    get_attribute, get_attribute_mut, has_attribute, is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::attribute::{path, presentation::LengthPercentage, uncategorised::ViewBox};
use oxvg_path::{command::Data, Path};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(transparent)]
/// For SVGs with a `viewBox` attribute, removes `<path>` element outside of it's bounds.
///
/// Elements with `transform` are ignored, as they may be affected by animations.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveOffCanvasPaths(pub bool);

struct State {
    view_box_data: RefCell<Option<ViewBox>>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveOffCanvasPaths {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State {
                view_box_data: RefCell::new(None),
            }
            .start_with_context(document, context)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if element.is_root() && is_element!(element, Svg) {
            self.view_box_data.replace(gather(element).ok());
        }

        if has_attribute!(element, Transform) {
            context.flags.visit_skip();
            return Ok(());
        }

        if !is_element!(element, Path) {
            return Ok(());
        }
        let view_box_data = self.view_box_data.borrow();
        let Some(view_box_data) = view_box_data.as_ref() else {
            return Ok(());
        };
        let mut path = get_attribute_mut!(element, D);
        let Some(path::Path(path)) = path.as_deref_mut() else {
            return Ok(());
        };

        let visible = path.0.iter().any(|c| match c.as_explicit() {
            Data::MoveTo([x, y]) => {
                *x >= view_box_data.min_x as f64
                    && *x <= (view_box_data.min_x + view_box_data.width) as f64
                    && *y >= view_box_data.min_y as f64
                    && *y <= (view_box_data.min_y + view_box_data.height) as f64
            }
            _ => false,
        });
        if visible {
            return Ok(());
        }

        if path.0.len() == 2 {
            path.0.push(Data::ClosePath);
        }
        let ViewBox {
            min_x,
            min_y,
            width,
            height,
        } = view_box_data;
        let view_box_path_data = Path(vec![
            Data::MoveTo([*min_x as f64, *min_y as f64]),
            Data::HorizontalLineBy([*width as f64]),
            Data::VerticalLineBy([*height as f64]),
            Data::HorizontalLineTo([*min_x as f64]),
            Data::ClosePath,
        ]);

        if !view_box_path_data.intersects(path) {
            element.remove();
        }
        Ok(())
    }
}

enum GatherViewboxDataError {
    ParseFloatError,
    MissingViewbox,
}

fn gather(element: &Element) -> Result<ViewBox, GatherViewboxDataError> {
    let width = get_attribute!(element, WidthSvg);
    let height = get_attribute!(element, HeightSvg);
    let Some(viewbox) = get_attribute!(element, ViewBox) else {
        match (width.as_deref(), height.as_deref()) {
            (
                Some(LengthPercentage(DimensionPercentage::Dimension(width))),
                Some(LengthPercentage(DimensionPercentage::Dimension(height))),
            ) => {
                return Ok(ViewBox {
                    min_x: 0.0,
                    min_y: 0.0,
                    width: width
                        .to_px()
                        .ok_or(GatherViewboxDataError::ParseFloatError)?,
                    height: height
                        .to_px()
                        .ok_or(GatherViewboxDataError::ParseFloatError)?,
                })
            }
            _ => return Err(GatherViewboxDataError::MissingViewbox),
        }
    };

    Ok(viewbox.clone())
}

#[test]
fn remove_off_canvas_paths() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeOffCanvasPaths": true }"#,
        Some(
            r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <path d="M10 10 h 80 v 80 h -80 z"/>
    <path d="M10 -90 h 80 v 80 h -80 z"/>
    <path d="M110 10 h 80 v 80 h -80 z"/>
    <path d="M10 110 h 80 v 80 h -80 z"/>
    <path d="M-90 10 h 80 v 80 h -80 z"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeOffCanvasPaths": true }"#,
        Some(
            r#"<svg height="1000" width="1000" xmlns="http://www.w3.org/2000/svg">
    <path d="M10 10 h 80 v 80 h -80 z"/>
    <path d="M10 -90 h 80 v 80 h -80 z"/>
    <path d="M110 10 h 80 v 80 h -80 z"/>
    <path d="M10 110 h 80 v 80 h -80 z"/>
    <path d="M-90 10 h 80 v 80 h -80 z"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeOffCanvasPaths": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
    <path d="M0 0h128v128H0z" fill="none" stroke="red"/>
    <path d="M10.14 51.5c4.07 1.56 7.52 4.47 7.37 11.16" fill="none" stroke="#00f"/>
    <path d="M100 200c4.07 1.56 7.52 4.47 7.37 11.16" fill="none" stroke="#00f"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeOffCanvasPaths": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
    <path d="M20.16 107.3l13.18-12.18m-1.6-5.41l-16.32 6.51M13 84.5h18m77 22.8L94.83 95.12m1.6-5.41l16.32 6.51M115 84.5H98" fill="none" stroke="#444" stroke-width="3"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeOffCanvasPaths": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <path d="M-100-100h50v50h-50z" fill="red" transform="translate(100 100)"/>
    <g transform="translate(150 150)">
        <path d="M-100-100h50v50h-50z" fill="blue"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeOffCanvasPaths": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M10 10 h 80 v 80 h -80 z"/>
    <path d="M10 -90 h 80 v 80 h -80 z"/>
    <path d="M110 10 h 80 v 80 h -80 z"/>
    <path d="M10 110 h 80 v 80 h -80 z"/>
    <path d="M-90 10 h 80 v 80 h -80 z"/>
</svg>"#
        ),
    )?);

    Ok(())
}
