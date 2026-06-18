use oxvg_ast::{
    get_computed_style,
    style::{ComputedStyles, Mode},
};
use oxvg_collections::attribute::inheritable::Inheritable;
use oxvg_path::optimize;

use lightningcss::{
    properties::svg::{Marker, SVGPaint, StrokeLinecap, StrokeLinejoin},
    values::shape::FillRule,
};

pub fn gather_optimize_options(computed_styles: &ComputedStyles) -> optimize::Options {
    let mut options = optimize::Options::all();

    let stroke = get_computed_style!(computed_styles, Stroke);
    let linecap = get_computed_style!(computed_styles, StrokeLinecap);
    let linejoin = get_computed_style!(computed_styles, StrokeLinejoin);
    let maybe_has_stroke = stroke.is_some_and(|(stroke, mode)| {
        mode == Mode::Dynamic || !matches!(stroke.option(), Some(SVGPaint::None))
    });
    let maybe_has_linecap = linecap.as_ref().is_some_and(|(linecap, mode)| {
        *mode == Mode::Static && !matches!(linecap, Inheritable::Defined(StrokeLinecap::Butt))
    });
    let safe_to_close = !maybe_has_stroke
        || (linecap.is_some_and(|(linecap, mode)| {
            mode == Mode::Static && matches!(linecap, Inheritable::Defined(StrokeLinecap::Round))
        }) && linejoin.is_some_and(|(linejoin, mode)| {
            mode == Mode::Static && matches!(linejoin, Inheritable::Defined(StrokeLinejoin::Round))
        }));

    let fill_rule = get_computed_style!(computed_styles, FillRule);
    let maybe_has_nonzero = fill_rule.as_ref().is_none_or(|(fill_rule, mode)| {
        *mode == Mode::Dynamic
            || matches!(
                fill_rule,
                Inheritable::Defined(FillRule::Nonzero) | Inheritable::Inherited
            )
    });
    let maybe_has_evenodd = fill_rule.is_some_and(|(fill_rule, mode)| {
        mode == Mode::Dynamic
            || matches!(
                fill_rule,
                Inheritable::Defined(FillRule::Evenodd) | Inheritable::Inherited
            )
    });

    let maybe_has_marker =
        get_computed_style!(computed_styles, Marker).is_some_and(|(marker, mode)| {
            mode == Mode::Dynamic
                || matches!(
                    marker,
                    Inheritable::Inherited | Inheritable::Defined(Marker::Url(_))
                )
        });
    let maybe_has_marker_start = maybe_has_marker
        || get_computed_style!(computed_styles, MarkerStart).is_some_and(|(marker, mode)| {
            mode == Mode::Dynamic
                || matches!(
                    marker,
                    Inheritable::Inherited | Inheritable::Defined(Marker::Url(_))
                )
        });
    let maybe_has_marker_mid = maybe_has_marker
        || get_computed_style!(computed_styles, MarkerMid).is_some_and(|(marker, mode)| {
            mode == Mode::Dynamic
                || matches!(
                    marker,
                    Inheritable::Inherited | Inheritable::Defined(Marker::Url(_))
                )
        });
    let maybe_has_marker_end = maybe_has_marker
        || get_computed_style!(computed_styles, MarkerEnd).is_some_and(|(marker, mode)| {
            mode == Mode::Dynamic
                || matches!(
                    marker,
                    Inheritable::Inherited | Inheritable::Defined(Marker::Url(_))
                )
        });
    let maybe_has_any_marker =
        maybe_has_marker_start || maybe_has_marker_mid || maybe_has_marker_end;

    // PERF: Branchless
    options &= !optimize::Options::UnsafeStroke
        | optimize::Options::from_bits_retain((!maybe_has_stroke as u16) * u16::MAX);
    options.set(optimize::Options::CloseSegments, safe_to_close);
    options &= !optimize::Options::UnsafeStrokeLinecap
        | optimize::Options::from_bits_retain(
            (!(maybe_has_stroke && maybe_has_linecap) as u16) * u16::MAX,
        );
    options &= !optimize::Options::UnsafeNonZero
        | optimize::Options::from_bits_retain((!maybe_has_nonzero as u16) * u16::MAX);
    options &= !optimize::Options::UnsafeEvenOdd
        | optimize::Options::from_bits_retain((!maybe_has_evenodd as u16) * u16::MAX);
    options &= !optimize::Options::UnsafeMarker
        | optimize::Options::from_bits_retain((!maybe_has_any_marker as u16) * u16::MAX);
    options &= !optimize::Options::UnsafeMarkerStart
        | optimize::Options::from_bits_retain((!maybe_has_marker_start as u16) * u16::MAX);
    options &= !optimize::Options::UnsafeMarkerMid
        | optimize::Options::from_bits_retain((!maybe_has_marker_mid as u16) * u16::MAX);
    options &= !optimize::Options::UnsafeMarkerEnd
        | optimize::Options::from_bits_retain((!maybe_has_marker_end as u16) * u16::MAX);
    options.set(optimize::Options::RemoveCloseLine, safe_to_close);

    options
}
