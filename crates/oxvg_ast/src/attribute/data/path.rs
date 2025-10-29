//! Path data attributes as specified in [paths](https://svgwg.org/svg2-draft/paths.html#DProperty)
use std::ops::Deref;

use oxvg_path::convert::StyleInfo;

use crate::{
    attribute::data::AttrId,
    element::Element,
    error::PrinterError,
    get_computed_style, has_attribute,
    serialize::{Printer, ToAtom},
    style::Mode,
};

#[derive(Clone, Debug, PartialEq)]
/// A set of path commands
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/paths.html#PathData)
/// [w3 | SVG 2](https://svgwg.org/svg2-draft/paths.html#DProperty)
pub struct Path(pub oxvg_path::Path);
impl Path {
    // TODO: implement ToCss compatible method
    /// Parse a value from a string
    ///
    /// # Errors
    /// If parsing fails
    pub fn parse_string(definition: &str) -> Result<Self, oxvg_path::parser::Error> {
        oxvg_path::Path::parse_string(definition).map(Self)
    }
}
impl ToAtom for Path {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        use std::fmt::Write;
        struct Writer<W: Write>(W);
        impl<W: Write> Write for Writer<W> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                self.0.write_str(s).map_err(|_| std::fmt::Error)
            }
        }

        write!(Writer(dest), "{}", self.0).map_err(|_| PrinterError {
            kind: lightningcss::error::PrinterErrorKind::FmtError,
            loc: None,
        })
    }
}

impl Deref for Path {
    type Target = oxvg_path::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Determine the path optimisations that are allowed based on relevant context
pub fn gather_style_info(
    element: &Element,
    computed_styles: &crate::style::ComputedStyles,
) -> StyleInfo {
    use crate::attribute::data::inheritable::Inheritable;
    use lightningcss::properties::svg::{SVGPaint, StrokeLinecap, StrokeLinejoin};

    let has_marker = has_attribute!(element, MarkerStart) || has_attribute!(element, MarkerEnd);
    let has_marker_mid = computed_styles.get(&AttrId::MarkerMid).is_some();

    let stroke = get_computed_style!(computed_styles, Stroke);
    let maybe_has_stroke = stroke
        .is_some_and(|(stroke, mode)| mode == Mode::Dynamic || !matches!(stroke, SVGPaint::None));

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
