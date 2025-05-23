use std::cell::Cell;

use lightningcss::{
    properties::{svg::SVGPaint, Property, PropertyId},
    traits::Zero,
    values::alpha::AlphaValue,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    get_computed_styles_factory,
    name::Name,
    style::{Id, PresentationAttr, PresentationAttrId, Static},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::{ElementGroup, Group};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Removes useless `stroke` and `fill` attributes
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
pub struct RemoveUselessStrokeAndFill {
    #[serde(default = "default_stroke")]
    /// Whether to remove redundant strokes
    pub stroke: bool,
    #[serde(default = "default_fill")]
    /// Whether to remove redundant fills
    pub fill: bool,
    #[serde(default = "default_remove_none")]
    /// Whether to remove elements with no stroke or fill
    pub remove_none: bool,
}

struct State<'o> {
    options: &'o RemoveUselessStrokeAndFill,
    id_rc_byte: Cell<Option<usize>>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveUselessStrokeAndFill {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        context_flags.query_has_script(document);
        context_flags.query_has_stylesheet(document);
        State {
            options: self,
            id_rc_byte: Cell::new(None),
        }
        .start(&mut document.clone(), info, Some(context_flags.clone()))?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'_> {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(
            if context_flags.intersects(ContextFlags::has_stylesheet | ContextFlags::has_script_ref)
            {
                PrepareOutcome::skip
            } else {
                PrepareOutcome::use_style
            },
        )
    }

    fn use_style(&self, element: &E) -> bool {
        if self.id_rc_byte.get().is_some() {
            return false;
        }

        if element.has_attribute_local(&"id".into()) {
            log::debug!("flagged as id root");
            self.id_rc_byte.set(Some(element.id()));
            return false;
        }

        let name = element.qual_name();
        name.prefix().is_none()
            && ElementGroup::Shape
                .set()
                .contains(name.local_name().as_ref())
    }

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if !context.flags.contains(ContextFlags::use_style) {
            log::debug!("use_style indicated non-removable context");
            return Ok(());
        }

        self.remove_stroke(element, context);
        self.remove_fill(element, context);
        Ok(())
    }

    fn exit_element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if self.id_rc_byte.get().is_some_and(|b| b == element.id()) {
            log::debug!("unflagged as id root");
            self.id_rc_byte.set(None);
        }

        Ok(())
    }
}

impl State<'_> {
    fn remove_stroke<'arena, E: Element<'arena>>(
        &self,
        element: &E,
        context: &mut Context<'arena, '_, '_, E>,
    ) {
        if !self.options.stroke {
            return;
        }

        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);

        let marker_end = get_computed_styles!(MarkerEnd);

        let stroke_width = get_computed_styles!(StrokeWidth);
        let is_stroke_width_zero = stroke_width.is_some_and(|s| {
            if s.is_dynamic() {
                return false;
            };
            if let Static::Css(Property::StrokeWidth(length))
            | Static::Attr(PresentationAttr::StrokeWidth(length)) = s.inner()
            {
                length.is_zero()
            } else {
                false
            }
        });

        if marker_end.is_some() && !is_stroke_width_zero {
            log::debug!("skipping stroke removal, has marker");
            return;
        }

        let stroke = get_computed_styles!(Stroke);
        let mut is_stroke_eq_none = stroke.is_some_and(|s| {
            matches!(
                s.inner(),
                Static::Attr(PresentationAttr::Stroke(SVGPaint::None))
                    | Static::Css(Property::Stroke(SVGPaint::None))
            )
        });
        let is_stroke_none = stroke.is_none_or(|s| s.is_static() && is_stroke_eq_none);

        let stroke_opacity = get_computed_styles!(StrokeOpacity);
        let is_stroke_opacity_zero = stroke_opacity.is_some_and(|s| {
            s.is_static()
                && matches!(
                    s.inner(),
                    Static::Attr(PresentationAttr::StrokeOpacity(AlphaValue(0.0)))
                        | Static::Css(Property::StrokeOpacity(AlphaValue(0.0)))
                )
        });

        let stroke_width = get_computed_styles!(StrokeWidth);
        let is_stroke_width_zero = stroke_width.is_some_and(|s| {
            if s.is_dynamic() {
                return false;
            };
            if let Static::Css(Property::StrokeWidth(length))
            | Static::Attr(PresentationAttr::StrokeWidth(length)) = s.inner()
            {
                length.is_zero()
            } else {
                false
            }
        });

        if is_stroke_none || is_stroke_opacity_zero || is_stroke_width_zero {
            log::debug!("removing useless stroke");
            log::debug!("stroke none: {is_stroke_none}");
            log::debug!("stroke opacity zero: {is_stroke_opacity_zero}");
            log::debug!("stroke width zero: {is_stroke_width_zero}");
            element.attributes().retain(|attr| {
                attr.prefix().is_some() || !attr.local_name().as_ref().starts_with("stroke")
            });

            if let Some(parent_stroke) = computed_styles
                .inherited
                .get(&Id::CSS(PropertyId::Stroke))
                .or_else(|| {
                    computed_styles
                        .inherited
                        .get(&Id::Attr(PresentationAttrId::Stroke))
                })
            {
                if parent_stroke.is_static()
                    && !matches!(
                        parent_stroke.inner(),
                        Static::Css(Property::Stroke(SVGPaint::None))
                            | Static::Attr(PresentationAttr::Stroke(SVGPaint::None))
                    )
                {
                    log::debug!("stroke is also inherited, setting to `none`");
                    element.set_attribute_local("stroke".into(), "none".into());
                    is_stroke_eq_none = true;
                }
            }
        }

        if is_stroke_eq_none && self.options.remove_none {
            log::debug!("removing element with no stroke");
            element.remove();
        }
    }

    fn remove_fill<'arena, E: Element<'arena>>(
        &self,
        element: &E,
        context: &mut Context<'arena, '_, '_, E>,
    ) {
        if !self.options.fill {
            return;
        }

        let computed_styles = &context.computed_styles;
        get_computed_styles_factory!(computed_styles);

        let fill = get_computed_styles!(Fill);
        let mut is_fill_eq_none = fill.is_some_and(|s| {
            matches!(
                s.inner(),
                Static::Css(Property::Fill(SVGPaint::None))
                    | Static::Attr(PresentationAttr::Fill(SVGPaint::None))
            )
        });
        let is_fill_none = fill.is_some_and(|s| s.is_static() && is_fill_eq_none);

        let fill_opacity = get_computed_styles!(FillOpacity);
        let is_fill_opacity_zero = fill_opacity.is_some_and(|s| {
            s.is_static()
                && matches!(
                    s.inner(),
                    Static::Css(Property::FillOpacity(AlphaValue(0.0)))
                        | Static::Attr(PresentationAttr::FillOpacity(AlphaValue(0.0)))
                )
        });

        if is_fill_none || is_fill_opacity_zero {
            log::debug!("removing useless fill");
            log::debug!("fill none: {is_fill_none}");
            log::debug!("fill opacity zero: {is_fill_opacity_zero}");
            element.attributes().retain(|attr| {
                attr.prefix().is_some() || !attr.local_name().as_ref().starts_with("fill-")
            });

            if fill.is_none() || !is_fill_eq_none {
                element.set_attribute_local("fill".into(), "none".into());
                is_fill_eq_none = true;
            }
        }

        if is_fill_eq_none && self.options.remove_none {
            log::debug!("removing element with no fill");
            element.remove();
        }
    }
}

impl Default for RemoveUselessStrokeAndFill {
    fn default() -> Self {
        RemoveUselessStrokeAndFill {
            stroke: default_stroke(),
            fill: default_fill(),
            remove_none: default_remove_none(),
        }
    }
}

const fn default_stroke() -> bool {
    true
}
const fn default_fill() -> bool {
    true
}
const fn default_remove_none() -> bool {
    false
}

#[test]
fn remove_useless_stroke_and_fill() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessStrokeAndFill": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- don't affect elements within id'd element -->
    <defs>
        <g id="test">
            <rect stroke-dashoffset="5" width="100" height="100"/>
        </g>
    </defs>
    <!-- remove useless strokes/fills -->
    <circle fill="red" stroke-width="6" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="0" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="6" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <!-- replace useless strokes with "none" when inherited stroke will replace it -->
    <g stroke="#000" stroke-width="6">
        <circle fill="red" stroke="red" stroke-width="0" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
        <circle fill="red" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    </g>
    <g stroke="#000">
        <circle fill="red" stroke-width="0" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
        <circle fill="red" stroke="none" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessStrokeAndFill": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove useless fills -->
    <defs>
        <g id="test">
            <rect fill-opacity=".5" width="100" height="100"/>
        </g>
    </defs>
    <circle fill="none" fill-rule="evenodd" cx="60" cy="60" r="50"/>
    <circle fill="red" fill-opacity="0" cx="90" cy="90" r="50"/>
    <circle fill-opacity="0" fill-rule="evenodd" cx="90" cy="60" r="50"/>
    <circle fill="red" fill-opacity=".5" cx="60" cy="60" r="50"/>
    <g fill="none">
        <circle fill-opacity=".5" cx="60" cy="60" r="50"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessStrokeAndFill": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- ignore documents with `style` -->
    <style>
        …
    </style>
    <circle fill="none" fill-rule="evenodd" cx="60" cy="60" r="50"/>
    <circle fill-opacity="0" fill-rule="evenodd" cx="90" cy="60" r="50"/>
    <circle fill="red" stroke-width="6" stroke-dashoffset="5" cx="60" cy="60" r="50"/>
    <circle fill="red" stroke="#000" stroke-width="6" stroke-dashoffset="5" stroke-opacity="0" cx="60" cy="60" r="50"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessStrokeAndFill": {} }"#,
        Some(
            r#"<svg width="480" height="360" xmlns="http://www.w3.org/2000/svg">
    <!-- don't remove stroke when useful stroke-width and marker-end is on element -->
    <defs>
        <marker id="testMarker">
            <rect width="100" height="100" fill="blue" />
        </marker>
    </defs>
    <line x1="150" y1="150" x2="165" y2="150" stroke="red" stroke-width="25" marker-end="url(#testMarker)" />
    <line x1="250" y1="250" x2="265" y2="250" stroke="red" stroke-width="0" marker-end="url(#testMarker)" />
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessStrokeAndFill": { "removeNone": true } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- remove element with useless stroke/fill -->
    <defs>
        <g id="test">
            <rect fill-opacity=".5" width="100" height="100"/>
        </g>
    </defs>
    <circle fill="none" fill-rule="evenodd" cx="60" cy="60" r="50"/>
    <circle fill="red" fill-opacity="0" cx="90" cy="90" r="50"/>
    <circle fill-opacity="0" fill-rule="evenodd" cx="90" cy="60" r="50"/>
    <circle fill="red" fill-opacity=".5" cx="60" cy="60" r="50"/>
    <g fill="none">
        <circle fill-opacity=".5" cx="60" cy="60" r="50"/>
    </g>
</svg>"#
        ),
    )?);

    Ok(())
}
