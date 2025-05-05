use std::cell::RefCell;

use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_path::{command::Data, Path};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Default, Debug)]
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
    view_box_data: RefCell<Option<ViewBoxData>>,
}

#[derive(Default, Clone, Debug)]
pub struct ViewBoxData {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveOffCanvasPaths {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State {
                view_box_data: RefCell::new(None),
            }
            .start(&mut document.clone(), info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if element.is_root() && element.prefix().is_none() && element.local_name().as_ref() == "svg"
        {
            self.view_box_data
                .replace(ViewBoxData::gather(element).ok());
        }

        if element.has_attribute_local(&"transform".into()) {
            context.flags.visit_skip();
            return Ok(());
        }

        if element.prefix().is_some() || element.local_name().as_ref() != "path" {
            return Ok(());
        }
        let view_box_data = self.view_box_data.borrow();
        let Some(view_box_data) = view_box_data.as_ref() else {
            return Ok(());
        };
        let Some(d) = element.get_attribute_local(&"d".into()) else {
            return Ok(());
        };
        let Ok(mut path) = oxvg_path::Path::parse(d.as_ref()) else {
            return Ok(());
        };

        let visible = path.0.iter().any(|c| match c.as_explicit() {
            Data::MoveTo([x, y]) => {
                x >= &view_box_data.left
                    && x <= &view_box_data.right
                    && y >= &view_box_data.top
                    && y <= &view_box_data.bottom
            }
            _ => false,
        });
        if visible {
            return Ok(());
        }

        if path.0.len() == 2 {
            path.0.push(Data::ClosePath);
        }
        let ViewBoxData {
            top,
            left,
            width,
            height,
            ..
        } = view_box_data.clone();
        let view_box_path_data = Path(vec![
            Data::MoveTo([left, top]),
            Data::HorizontalLineBy([width]),
            Data::VerticalLineBy([height]),
            Data::HorizontalLineTo([left]),
            Data::ClosePath,
        ]);

        if !view_box_path_data.intersects(&path) {
            element.remove();
        }
        Ok(())
    }
}

enum GatherViewboxDataError {
    ParseFloatError,
    MissingParameter,
    MissingViewbox,
}

impl ViewBoxData {
    fn gather<'arena, E: Element<'arena>>(element: &mut E) -> Result<Self, GatherViewboxDataError> {
        let width = element.get_attribute_local(&"width".into());
        let height = element.get_attribute_local(&"height".into());
        let Some(viewbox) = element.get_attribute_local(&"viewBox".into()) else {
            match (width, height) {
                (Some(width), Some(height)) => {
                    return Ok(ViewBoxData::fallback(
                        width
                            .as_ref()
                            .trim()
                            .strip_suffix("px")
                            .unwrap_or(width.as_ref())
                            .parse()
                            .map_err(|_| GatherViewboxDataError::ParseFloatError)?,
                        height
                            .as_ref()
                            .trim()
                            .strip_suffix("px")
                            .unwrap_or(height.as_ref())
                            .parse()
                            .map_err(|_| GatherViewboxDataError::ParseFloatError)?,
                    ))
                }
                _ => return Err(GatherViewboxDataError::MissingViewbox),
            }
        };

        let mut viewbox = viewbox
            .as_ref()
            .split_whitespace()
            .flat_map(|s| s.split(','))
            .map(str::trim)
            .map(|s| s.strip_suffix("px").unwrap_or(s))
            .map(str::parse::<f64>)
            .map(|r| r.map_err(|_| GatherViewboxDataError::ParseFloatError));

        let left = viewbox
            .next()
            .ok_or(GatherViewboxDataError::MissingParameter)??;
        let top = viewbox
            .next()
            .ok_or(GatherViewboxDataError::MissingParameter)??;
        let width = viewbox
            .next()
            .ok_or(GatherViewboxDataError::MissingParameter)??;
        let height = viewbox
            .next()
            .ok_or(GatherViewboxDataError::MissingParameter)??;

        Ok(ViewBoxData {
            left,
            top,
            right: left + width,
            bottom: top + height,
            width,
            height,
        })
    }
}

impl<'de> Deserialize<'de> for RemoveOffCanvasPaths {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let enabled = bool::deserialize(deserializer)?;
        Ok(Self(enabled))
    }
}

impl Serialize for RemoveOffCanvasPaths {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl ViewBoxData {
    fn fallback(width: f64, height: f64) -> Self {
        Self {
            right: width,
            bottom: height,
            width,
            height,
            ..Self::default()
        }
    }
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
