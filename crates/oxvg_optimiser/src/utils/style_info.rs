use oxvg_ast::{
    element::Element,
    get_computed_style, has_attribute, has_computed_style,
    style::{ComputedStyles, Mode},
};
use oxvg_collections::attribute::inheritable::Inheritable;
use oxvg_path::convert::StyleInfo;

use lightningcss::properties::svg::{SVGPaint, StrokeLinecap, StrokeLinejoin};

/// Determine the path optimisations that are allowed based on relevant context
pub fn gather_style_info(element: &Element, computed_styles: &ComputedStyles) -> StyleInfo {
    let has_marker = has_attribute!(element, MarkerStart | MarkerEnd);
    let has_marker_mid = has_computed_style!(computed_styles, MarkerMid);

    let stroke = get_computed_style!(computed_styles, Stroke);
    let maybe_has_stroke = stroke.is_some_and(|(stroke, mode)| {
        mode == Mode::Dynamic || !matches!(stroke.option(), Some(SVGPaint::None))
    });

    let linecap = get_computed_style!(computed_styles, StrokeLinecap);
    let maybe_has_linecap = linecap.as_ref().is_some_and(|(linecap, mode)| {
        *mode == Mode::Dynamic || !matches!(linecap, Inheritable::Defined(StrokeLinecap::Butt))
    });

    let linejoin = get_computed_style!(computed_styles, StrokeLinejoin);
    let is_safe_to_use_z = if maybe_has_stroke {
        linecap.is_some_and(|(property, mode)| {
            mode == Mode::Static && matches!(property, Inheritable::Defined(StrokeLinecap::Round))
        }) && linejoin.is_some_and(|(property, mode)| {
            mode == Mode::Static && matches!(property, Inheritable::Defined(StrokeLinejoin::Round))
        })
    } else {
        true
    };

    let mut result = StyleInfo::empty();
    result.set(StyleInfo::has_marker_mid, has_marker_mid);
    result.set(StyleInfo::maybe_has_stroke, maybe_has_stroke);
    result.set(StyleInfo::maybe_has_linecap, maybe_has_linecap);
    result.set(StyleInfo::is_safe_to_use_z, is_safe_to_use_z);
    result.set(StyleInfo::has_marker, has_marker);
    result
}
