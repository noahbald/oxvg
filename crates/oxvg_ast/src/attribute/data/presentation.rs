use super::core::{Name, Number, Percentage, IRI};
use lightningcss::values::length::LengthValue;
pub use lightningcss::{
    properties::{
        display::{Display, Visibility},
        effects::FilterList,
        font::{Font, FontFamily, FontSize, FontStretch, FontStyle, FontWeight},
        masking::{ClipPath, Mask},
        overflow::Overflow,
        svg::{
            ColorInterpolation, Marker, ShapeRendering, StrokeDasharray, StrokeLinecap,
            StrokeLinejoin,
        },
        text::{Direction, Spacing, TextDecoration, UnicodeBidi},
        ui::Cursor,
    },
    values::{
        length::{LengthOrNumber, LengthPercentage},
        shape::FillRule,
    },
};
use smallvec::{smallvec, SmallVec};

use crate::{
    enum_attr,
    error::{ParseError, ParseErrorKind, PrinterError},
    parse::{Parse, Parser},
    serialize::{Printer, ToAtom},
};

enum_attr!(
    /// The `alignment-baseline` attribute specifies how an object is aligned with respect to its parent.
    AlignmentBaseline {
        /// The value is the dominant-baseline of the script to which the character belongs - i.e., use the dominant-baseline of the parent.
        Auto: "auto",
        /// Uses the dominant baseline choice of the parent. Matches the box's corresponding [baseline](https://developer.mozilla.org/en-US/docs/Glossary/Baseline/Typography) to that of its parent.
        Baseline: "baseline",
        /// The alignment-point of the object being aligned is aligned with the "before-edge" baseline of the parent text content element.
        BeforeEdge: "before-edge",
        /// The alignment-point of the object being aligned is aligned with the "text-before-edge" baseline of the parent text content element.
        ///
        /// > [!NOTE] This keyword may be mapped to `text-top`.
        TextBeforeEdge: "text-before-edge",
        /// Aligns the vertical midpoint of the box with the baseline of the parent box plus half the x-height of the parent.
        Middle: "middle",
        /// Matches the box's central baseline to the central baseline of its parent.
        Central: "central",
        /// The alignment-point of the object being aligned is aligned with the "after-edge" baseline of the parent text content element.
        AfterEdge: "after-edge",
        /// The alignment-point of the object being aligned is aligned with the "text-after-edge" baseline of the parent text content element.
        ///
        /// > [!NOTE] This keyword may be mapped to `text-bottom`.
        TextAfterEdge: "text-after-edge",
        /// Matches the box's ideographic character face under-side baseline to that of its parent.
        Ideographic: "ideographic",
        /// Matches the box's alphabetic baseline to that of its parent.
        Alphabetic: "alphabetic",
        /// The alignment-point of the object being aligned is aligned with the "hanging" baseline of the parent text content element.
        Hanging: "hanging",
        /// Matches the box's mathematical baseline to that of its parent.
        Mathematical: "mathematical",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// The baseline-shift attribute allows repositioning of the dominant-baseline relative to the dominant-baseline of the parent text content element.
pub enum BaselineShift {
    /// A length value raises (positive value) or lowers (negative value) the dominant-baseline of the parent text content element by the specified length.
    ///
    /// A percentage value raises (positive value) or lowers (negative value) the dominant-baseline of the parent text content element by the specified percentage of the `line-height`.
    Baseline,
    /// The dominant-baseline is shifted to the default position for subscripts.
    Sub,
    /// The dominant-baseline is shifted to the default position for superscripts.
    Super,
    Top,
    Center,
    Bottom,
    Percentage(Percentage),
    Length(LengthValue),
}
impl<'input> Parse<'input> for BaselineShift {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                Ok(match ident {
                    "baseline" => Self::Baseline,
                    "sub" => Self::Sub,
                    "super" => Self::Super,
                    "top" => Self::Top,
                    "center" => Self::Center,
                    "bottom" => Self::Bottom,
                    _ => return Err(()),
                })
            })
            .or_else(|_| input.try_parse(Percentage::parse).map(Self::Percentage))
            .or_else(|_| LengthValue::parse(input).map(Self::Length))
    }
}
impl ToAtom for BaselineShift {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Baseline => dest.write_char('0'),
            Self::Sub => dest.write_str("sub"),
            Self::Super => dest.write_str("super"),
            Self::Top => dest.write_str("top"),
            Self::Center => dest.write_str("center"),
            Self::Bottom => dest.write_str("bottom"),
            Self::Percentage(percentage) => percentage.write_atom(dest),
            Self::Length(length) => length.write_atom(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Clip {
    /// Only valid `<shape>` is `rect(<top> <right> <bottom> <left>)`
    Shape([Number; 4]),
    Auto,
}
impl<'input> Parse<'input> for Clip {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| input.expect_ident_matching("auto").map(|_| Self::Auto))
            .or_else(|_| {
                input.expect_function_matching("rect")?;
                input.parse_nested_block(|input| {
                    input.skip_whitespace();
                    let top = Number::parse(input)?;
                    input.skip_whitespace();
                    let right = Number::parse(input)?;
                    input.skip_whitespace();
                    let bottom = Number::parse(input)?;
                    input.skip_whitespace();
                    let left = Number::parse(input)?;
                    input.skip_whitespace();
                    Ok(Self::Shape([top, right, bottom, left]))
                })
            })
    }
}
impl ToAtom for Clip {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::Shape([top, right, bottom, left]) => {
                dest.write_str("rect(")?;
                top.write_atom(dest)?;
                dest.write_char(' ')?;
                right.write_atom(dest)?;
                dest.write_char(' ')?;
                bottom.write_atom(dest)?;
                dest.write_char(' ')?;
                left.write_atom(dest)?;
                dest.write_char(')')
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorProfile<'i> {
    Auto,
    SRGB,
    Name(Name<'i>),
    IRI(IRI<'i>),
}
impl<'input> Parse<'input> for ColorProfile<'input> {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                Ok(match ident {
                    "auto" => Self::Auto,
                    "sRGB" => Self::SRGB,
                    _ => return Err(()),
                })
            })
            .or_else(|_| input.try_parse(Name::parse).map(Self::Name))
            .or_else(|_| IRI::parse(input).map(Self::IRI))
    }
}
impl ToAtom for ColorProfile<'_> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::SRGB => dest.write_str("sRGB"),
            Self::Name(name) => name.write_atom(dest),
            Self::IRI(iri) => iri.write_atom(dest),
        }
    }
}

enum_attr!(Rendering {
    Auto: "auto",
    OptimizeSpeed: "optimizeSpeed",
    OptimizeQuality: "optimizeQuality",
    Inherit: "inherit",
});
enum_attr!(DominantBaseline {
    Auto: "auto",
    UseScript: "use-script",
    NoChange: "no-change",
    ResetSize: "reset-size",
    Ideographic: "ideographic",
    Alphabetic: "alphabetic",
    Hanging: "hanging",
    Mathematical: "mathematical",
    Central: "central",
    Middle: "middle",
    TextAfterEdge: "text-after-edge",
    TextBeforeEdge: "text-before-edge",
    Inherit: "inherit",
});

#[derive(Clone, Debug, PartialEq)]
pub enum EnableBackground {
    Accumulate,
    New {
        x: Number,
        y: Number,
        width: Number,
        height: Number,
    },
}
impl<'input> Parse<'input> for EnableBackground {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("accumulate")
                    .map(|_| Self::Accumulate)
            })
            .or_else(|_| {
                input.expect_ident_matching("new");
                input.skip_whitespace();
                let x = Number::parse(input)?;
                input.skip_whitespace();
                let y = Number::parse(input)?;
                input.skip_whitespace();
                let width = Number::parse(input)?;
                input.skip_whitespace();
                let height = Number::parse(input)?;
                Ok(Self::New {
                    x,
                    y,
                    width,
                    height,
                })
            })
    }
}
impl ToAtom for EnableBackground {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Accumulate => dest.write_str("accumulate"),
            Self::New {
                x,
                y,
                width,
                height,
            } => {
                dest.write_str("new ")?;
                x.write_atom(dest)?;
                dest.write_char(' ')?;
                y.write_atom(dest)?;
                dest.write_char(' ')?;
                width.write_atom(dest)?;
                dest.write_char(' ')?;
                height.write_atom(dest)
            }
        }
    }
}

enum_attr!(FontVariant {
    Normal: "normal",
    SmallCaps: "small-caps",
    Inherit: "inherit",
});

enum_attr!(Paint {
    Stroke: "stroke",
    Fill: "fill",
    Markers: "markers",
});

/// Controls the order in which the steps of painting are done
///
/// [MDN | paint-order](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/paint-order)
#[derive(Debug, PartialEq, Clone)]
pub struct PaintOrder(pub SmallVec<[Paint; 3]>);
impl PaintOrder {
    fn normal() -> Self {
        Self(smallvec![Paint::Fill, Paint::Stroke, Paint::Markers])
    }
    fn is_normal(&self) -> bool {
        let inner = &self.0;
        inner[0] == Paint::Fill && inner[1] == Paint::Stroke && inner[2] == Paint::Markers
    }
}
impl<'input> Parse<'input> for PaintOrder {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        let normal = Self::normal();
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(normal);
        }
        let mut paint_order = SmallVec::with_capacity(3);
        for _ in 0..3 {
            if let Ok(paint) = input.try_parse(Paint::parse) {
                paint_order.push(paint);
            } else {
                break;
            }
        }
        if paint_order.is_empty() {
            let location = input.current_source_location();
            return Err(location.new_custom_error(ParseErrorKind::InvalidPaintOrder));
        }
        for paint in normal.0 {
            if !paint_order.contains(&paint) {
                paint_order.push(paint);
            }
        }
        debug_assert_eq!(paint_order.len(), 3);
        Ok(Self(paint_order))
    }
}
impl ToAtom for PaintOrder {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        assert_eq!(self.0.len(), 3);
        if self.is_normal() {
            return dest.write_str("normal");
        }
        self.0[0].write_atom(dest)?;
        dest.write_char(' ')?;
        self.0[1].write_atom(dest)?;
        dest.write_char(' ')?;
        self.0[2].write_atom(dest)
    }
}

pub type Position = lightningcss::values::position::Position;

enum_attr!(PointerEvents {
    VisiblePainted: "visiblePainted",
    VisibleFill: "visibleFill",
    VisibleStroke: "visibleStroke",
    Visible: "visible",
    Painted: "painted",
    Fill: "fill",
    Stroke: "stroke",
    All: "all",
    None: "none",
    Inherit: "inherit",
});
enum_attr!(TextAnchor {
    Start: "start",
    Middle: "middle",
    End: "end",
});

enum_attr!(
    /// The vector effect to use when drawing an object
    ///
    /// [MDN | vector-effect](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/vector-effect)
    VectorEffect {
        /// This value specifies that no vector effect shall be applied
        None: "none",
        /// The resulting visual effect of this value is that the stroke width is not dependent on the transformations of the element (including non-uniform scaling and shear transformations) and zoom level.
        NonScalingStroke: "non-scaling-stroke",
        /// The scale of that user coordinate system does not change in spite of any transformation changes from a host coordinate space.
        NonScalingSize: "non-scaling-size",
        /// The rotation and skew of that user coordinate system is suppressed in spite of any transformation changes from a host coordinate space.
        NonRotation: "non-rotation",
        /// The position of user coordinate system is fixed in spite of any transformation changes from a host coordinate space.
        FixedPosition: "fixed-position",
    }
);

enum_attr!(WritingMode {
    LrTb: "lr-tb"
    RlTb: "rl-tb"
    TbRl: "tb-rl"
    Lr: "lr"
    Rl: "rl"
    Tb: "tb"
});
