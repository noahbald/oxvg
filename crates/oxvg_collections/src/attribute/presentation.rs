//! Presentation attributes as specified in [styling](https://svgwg.org/svg2-draft/attindex.html#PresentationAttributes)
use std::ops::{Deref, DerefMut};

use super::{
    core::{Angle, Length, Name, Number, Percentage, IRI},
    list_of::{Comma, ListOf},
};
use lightningcss::values::length::LengthValue;
pub use lightningcss::{
    properties::{
        display::{Display, Visibility},
        effects::FilterList,
        font::{Font, FontSize, FontStretch, FontStyle, FontWeight},
        masking::{ClipPath, MaskType},
        overflow::Overflow,
        svg::{
            ColorInterpolation, ColorRendering, ImageRendering, Marker, ShapeRendering,
            StrokeDasharray, StrokeLinecap, StrokeLinejoin, TextRendering,
        },
        text::{Direction, Spacing, TextDecoration, UnicodeBidi},
        ui::Cursor,
    },
    values::{length::LengthOrNumber, shape::FillRule},
};
#[cfg(feature = "parse")]
use oxvg_parse::{
    error::{ParseError, ParseErrorKind},
    Parse, Parser,
};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};
use smallvec::{smallvec, SmallVec};

use crate::enum_attr;

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
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/text.html#BaselineShiftProperty)
/// [w3 | SVG 2](https://svgwg.org/svg2-draft/text.html#BaselineShiftProperty)
pub enum BaselineShift {
    /// A length value raises (positive value) or lowers (negative value) the dominant-baseline of the parent text content element by the specified length.
    ///
    /// A percentage value raises (positive value) or lowers (negative value) the dominant-baseline of the parent text content element by the specified percentage of the `line-height`.
    Baseline,
    /// The dominant-baseline is shifted to the default position for subscripts.
    Sub,
    /// The dominant-baseline is shifted to the default position for superscripts.
    Super,
    /// Align the top edge if the inline box with the top edge of the parent box
    Top,
    /// Align the centre if the inline box with the centre of the parent box
    Center,
    /// Align the bottom edge if the inline box with the bottom edge of the parent box
    Bottom,
    /// Raise or lower by a percentage relative to the line-height
    Percentage(Percentage),
    /// Raise or lower by a specified length
    Length(LengthValue),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for BaselineShift {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = input.expect_ident().map_err(|_| ())?;
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
            .or_else(|()| input.try_parse(Percentage::parse).map(Self::Percentage))
            .or_else(|_| LengthValue::parse(input).map(Self::Length))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for BaselineShift {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
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
            Self::Percentage(percentage) => percentage.write_value(dest),
            Self::Length(length) => length.write_value(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Clips the paint area of an element
///
/// [w3](https://www.w3.org/TR/2011/REC-SVG11-20110816/masking.html#ClipProperty)
pub enum Clip {
    /// A shape specifying the area to clip.
    ///
    /// Only valid `<shape>` is `rect(<top> <right> <bottom> <left>)`
    Shape([Number; 4]),
    /// Clips along the bounds of the viewport
    Auto,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Clip {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| input.expect_ident_matching("auto").map(|()| Self::Auto))
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
#[cfg(feature = "serialize")]
impl ToValue for Clip {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::Shape([top, right, bottom, left]) => {
                dest.write_str("rect(")?;
                top.write_value(dest)?;
                dest.write_char(' ')?;
                right.write_value(dest)?;
                dest.write_char(' ')?;
                bottom.write_value(dest)?;
                dest.write_char(' ')?;
                left.write_value(dest)?;
                dest.write_char(')')
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Controls the color space of the image
///
/// [w3](https://www.w3.org/TR/2011/REC-SVG11-20110816/color.html#ColorProfileProperty)
pub enum ColorProfile<'i> {
    /// Follows the user-agents default colour profile
    Auto,
    /// The colour profile is assumed to be sRGB
    SRGB,
    /// A name within the user-agent's colour profile description database
    Name(Name<'i>),
    /// A reference to the colour profile
    IRI(IRI<'i>),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for ColorProfile<'input> {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = input.expect_ident().map_err(|_| ())?;
                if ident.to_lowercase() == "srgb" {
                    return Ok(Self::SRGB);
                }
                Ok(match ident {
                    "auto" => Self::Auto,
                    _ => return Err(()),
                })
            })
            .or_else(|()| input.try_parse(Name::parse).map(Self::Name))
            .or_else(|_| IRI::parse(input).map(Self::IRI))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for ColorProfile<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::SRGB => dest.write_str("sRGB"),
            Self::Name(name) => name.write_value(dest),
            Self::IRI(iri) => iri.write_value(dest),
        }
    }
}

enum_attr!(
    /// Specifies the baseline type used to align content within the box
    ///
    /// [w3](https://svgwg.org/svg2-draft/text.html#DominantBaselineProperty)
    DominantBaseline {
        /// Controls the baseline based on the horizontal and vertical writing modes
        Auto: "auto",
        /// Selects the baseline based on the characters data of the element
        UseScript: "use-script",
        /// Remains the same as the parent text content element
        NoChange: "no-change",
        /// Rescales the baseline to the current font-size
        ResetSize: "reset-size",
        /// Use the line-under design of CJK text
        Ideographic: "ideographic",
        /// Use the basline for Latin, Cyrillic, Greek, etc. text
        Alphabetic: "alphabetic",
        /// Use the baseline for Tibetan and similar scripts
        Hanging: "hanging",
        /// Use the baseline for mathematical characters
        Mathematical: "mathematical",
        /// Use the central baseline
        Central: "central",
        /// Use the halfway point between the alphabet's baseline and it's height
        Middle: "middle",
        /// Selects baseline based on font-size
        TextAfterEdge: "text-after-edge",
        /// Selects baseline based on font-size
        TextBeforeEdge: "text-before-edge",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// Specifies how the SVG manages the accumulation of the background image
pub enum EnableBackground {
    /// If an ancestor has `enable-background="new ..."`, then the current
    /// continer is rendered into parent's background and to the target device.
    Accumulate,
    /// Enables children to access a new background image
    New(Option<(Number, Number, Number, Number)>),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for EnableBackground {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("accumulate")
                    .map(|()| Self::Accumulate)
            })
            .or_else(|_| {
                input.expect_ident_matching("new")?;
                input.skip_whitespace();
                if let Ok(x) = input.try_parse(Number::parse) {
                    input.skip_whitespace();
                    let y = Number::parse(input)?;
                    input.skip_whitespace();
                    let width = Number::parse(input)?;
                    input.skip_whitespace();
                    let height = Number::parse(input)?;
                    Ok(Self::New(Some((x, y, width, height))))
                } else {
                    Ok(Self::New(None))
                }
            })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for EnableBackground {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Accumulate => dest.write_str("accumulate"),
            Self::New(None) => dest.write_str("new"),
            Self::New(Some((x, y, width, height))) => {
                dest.write_str("new ")?;
                x.write_value(dest)?;
                dest.write_char(' ')?;
                y.write_value(dest)?;
                dest.write_char(' ')?;
                width.write_value(dest)?;
                dest.write_char(' ')?;
                height.write_value(dest)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Indicates which font family is to be used to render the text
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/text.html#FontFamilyProperty)
pub struct FontFamily<'input>(
    pub ListOf<lightningcss::properties::font::FontFamily<'input>, Comma>,
);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for FontFamily<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        Ok(Self(ListOf::parse(input)?))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for FontFamily<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_value(dest)
    }
}

#[derive(Clone, Debug, PartialEq)]
/// An aspect value for an element that will preserve the x-height
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/text.html#FontSizeAdjustProperty)
pub enum FontSizeAdjust {
    /// The font's x-height
    Number(Number),
    /// Use the default x-height
    None,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for FontSizeAdjust {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| input.expect_ident_matching("none").map(|()| Self::None))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for FontSizeAdjust {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_value(dest),
            Self::None => dest.write_str("none"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Shorthand for font-variant sub-properties. Enables/disables various open-type features.
///
/// [w3](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant)
pub enum FontVariant {
    /// Resets all variants to their initial value
    Normal,
    /// Disables ligatures
    None,
    /// A set of font-variant coomponents
    Some {
        /// Controls ligatures
        font_variant_ligatures: FontVariantLigatures,
        /// Controls vaps
        font_variant_caps: Option<FontVariantCaps>,
        /// Controls numeric
        font_variant_numeric: FontVariantNumeric,
        /// Controls East Asian
        font_variant_east_asian: FontVariantEastAsian,
        /// Controls position
        font_variant_position: Option<FontVariantPosition>,
        /// Controls Emoji
        font_variant_emoji: Option<FontVariantEmoji>,
    },
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for FontVariant {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| {
                let str: &str = input.expect_ident().map_err(|_| ())?;
                Ok(match str {
                    "normal" => Self::Normal,
                    "none" => Self::None,
                    _ => return Err(()),
                })
            })
            .or_else(|()| {
                let mut font_variant_ligatures: Option<FontVariantLigatures> = None;
                let mut font_variant_caps: Option<FontVariantCaps> = None;
                let mut font_variant_numeric: Option<FontVariantNumeric> = None;
                let mut font_variant_east_asian: Option<FontVariantEastAsian> = None;
                let mut font_variant_position: Option<FontVariantPosition> = None;
                let mut font_variant_emoji: Option<FontVariantEmoji> = None;
                loop {
                    if font_variant_ligatures.is_none() {
                        if let Ok(value) = input.try_parse(FontVariantLigatures::parse) {
                            font_variant_ligatures = Some(value);
                            continue;
                        }
                    }
                    if font_variant_caps.is_none() {
                        if let Ok(value) = input.try_parse(FontVariantCaps::parse) {
                            font_variant_caps = Some(value);
                            continue;
                        }
                    }
                    if font_variant_numeric.is_none() {
                        if let Ok(value) = input.try_parse(FontVariantNumeric::parse) {
                            font_variant_numeric = Some(value);
                            continue;
                        }
                    }
                    if font_variant_east_asian.is_none() {
                        if let Ok(value) = input.try_parse(FontVariantEastAsian::parse) {
                            font_variant_east_asian = Some(value);
                            continue;
                        }
                    }
                    if font_variant_position.is_none() {
                        if let Ok(value) = input.try_parse(FontVariantPosition::parse) {
                            font_variant_position = Some(value);
                            continue;
                        }
                    }
                    if font_variant_emoji.is_none() {
                        if let Ok(value) = input.try_parse(FontVariantEmoji::parse) {
                            font_variant_emoji = Some(value);
                            continue;
                        }
                    }
                    break;
                }
                Ok(Self::Some {
                    font_variant_ligatures: font_variant_ligatures.unwrap_or_default(),
                    font_variant_caps,
                    font_variant_numeric: font_variant_numeric.unwrap_or_default(),
                    font_variant_east_asian: font_variant_east_asian.unwrap_or_default(),
                    font_variant_position,
                    font_variant_emoji,
                })
            })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for FontVariant {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Normal => dest.write_str("normal"),
            Self::None => dest.write_str("none"),
            Self::Some {
                font_variant_ligatures,
                font_variant_caps,
                font_variant_numeric,
                font_variant_east_asian,
                font_variant_position,
                font_variant_emoji,
            } => {
                let mut after = false;
                if *font_variant_ligatures != FontVariantLigatures::default() {
                    font_variant_ligatures.write_value(dest)?;
                    after = true;
                }
                if let Some(value) = font_variant_caps {
                    if after {
                        dest.write_char(' ')?;
                    }
                    value.write_value(dest)?;
                    after = true;
                }
                if *font_variant_numeric != FontVariantNumeric::default() {
                    if after {
                        dest.write_char(' ')?;
                    }
                    font_variant_numeric.write_value(dest)?;
                    after = true;
                }
                if *font_variant_east_asian != FontVariantEastAsian::default() {
                    if after {
                        dest.write_char(' ')?;
                    }
                    font_variant_east_asian.write_value(dest)?;
                    after = true;
                }
                if let Some(value) = font_variant_position {
                    if after {
                        dest.write_char(' ')?;
                    }
                    value.write_value(dest)?;
                    after = true;
                }
                if let Some(value) = font_variant_emoji {
                    if after {
                        dest.write_char(' ')?;
                    }
                    value.write_value(dest)?;
                }
                Ok(())
            }
        }
    }
}

/// Enables/disables various open-type ligature features.
///
/// [w3](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FontVariantLigatures {
    common_lig_values: Option<CommonLigValues>,
    discretionary_lig_values: Option<DiscretionaryLigValues>,
    historical_lig_values: Option<HistoricalLigValues>,
    contextual_alt_values: Option<ContextualAltValues>,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for FontVariantLigatures {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let mut result = FontVariantLigatures {
            common_lig_values: None,
            discretionary_lig_values: None,
            historical_lig_values: None,
            contextual_alt_values: None,
        };
        loop {
            if result.common_lig_values.is_none() {
                if let Ok(value) = input.try_parse(CommonLigValues::parse) {
                    result.common_lig_values = Some(value);
                    continue;
                }
            }
            if result.discretionary_lig_values.is_none() {
                if let Ok(value) = input.try_parse(DiscretionaryLigValues::parse) {
                    result.discretionary_lig_values = Some(value);
                    continue;
                }
            }
            if result.historical_lig_values.is_none() {
                if let Ok(value) = input.try_parse(HistoricalLigValues::parse) {
                    result.historical_lig_values = Some(value);
                    continue;
                }
            }
            if result.contextual_alt_values.is_none() {
                if let Ok(value) = input.try_parse(ContextualAltValues::parse) {
                    result.contextual_alt_values = Some(value);
                    continue;
                }
            }
            break;
        }
        Ok(result)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for FontVariantLigatures {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        if let Some(value) = &self.common_lig_values {
            value.write_value(dest)?;
        }
        if let Some(value) = &self.discretionary_lig_values {
            value.write_value(dest)?;
        }
        if let Some(value) = &self.historical_lig_values {
            value.write_value(dest)?;
        }
        if let Some(value) = &self.contextual_alt_values {
            value.write_value(dest)?;
        }
        Ok(())
    }
}
enum_attr!(
    /// Enables/disables `liga` and `clig`
    CommonLigValues {
        /// Enables `liga` and `clig`
        CommonLigatures: "common-ligatures",
        /// Disables `liga` and `clig`
        NoCommonLigatures: "no-common-ligatures",
    }
);
enum_attr!(
    /// Enables/disables `dlig`
    DiscretionaryLigValues {
        /// Enabled `dlig`
        DiscretionaryLigatures: "discretionary-ligatures",
        /// Disables `dlig`
        NoDiscretionaryLigatures: "no-discretionary-ligatures",
    }
);
enum_attr!(
    /// Enables/disables `hlig`
    HistoricalLigValues {
        /// Enables `hlig`
        HistoricalLigatures: "historical-ligatures",
        /// Disables `hlig`
        NoHistoricalLigatures: "no-historical-ligatures" ,
    }
);
enum_attr!(
    /// Enables/disables `calt`
    ContextualAltValues {
        /// Enables `calt`
        Contextual: "contextual",
        /// Disables `calt`
        NoContextual: "no-contextual",
    }
);

enum_attr!(
    /// Enables/disables various open-type capital features.
    ///
    /// [w3](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant)
    FontVariantCaps {
        /// Enables `smcp`
        SmallCaps: "small-caps",
        /// Enables `c2sc`, `smcp`
        AllSmallCaps: "all-small-caps",
        /// Enables `pcap`
        PetiteCaps: "petite-caps",
        /// Enables `c2pc`, `pcap`
        AllPetiteCaps: "all-petite-caps",
        /// Enables `unic`
        Unicase: "unicase",
        /// Enables `titl`
        TitlingCaps: "titling-caps",
    }
);

#[derive(Clone, Debug, Default, PartialEq)]
/// Enables/disables various open-type numeric features.
///
/// [w3](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant)
pub struct FontVariantNumeric {
    /// Numeric figure values
    numeric_figure_values: Option<NumericFigureValues>,
    /// Numeric spacing values
    numeric_spacing_values: Option<NumericSpacingValues>,
    /// Numeric fraction values
    numeric_fraction_values: Option<NumericFractionValues>,
    /// Enables `ordn`
    ordinal: bool,
    /// Enables `zero`
    slashed_zero: bool,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for FontVariantNumeric {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let mut result = Self {
            numeric_figure_values: None,
            numeric_spacing_values: None,
            numeric_fraction_values: None,
            ordinal: false,
            slashed_zero: false,
        };
        loop {
            if result.numeric_figure_values.is_none() {
                if let Ok(value) = input.try_parse(NumericFigureValues::parse) {
                    result.numeric_figure_values = Some(value);
                    continue;
                }
            }
            if result.numeric_spacing_values.is_none() {
                if let Ok(value) = input.try_parse(NumericSpacingValues::parse) {
                    result.numeric_spacing_values = Some(value);
                    continue;
                }
            }
            if result.numeric_fraction_values.is_none() {
                if let Ok(value) = input.try_parse(NumericFractionValues::parse) {
                    result.numeric_fraction_values = Some(value);
                    continue;
                }
            }
            if !result.ordinal {
                result.ordinal = input
                    .try_parse(|input| input.expect_ident_matching("ordinal"))
                    .is_ok();
                if result.ordinal {
                    continue;
                }
            }
            if !result.slashed_zero {
                result.slashed_zero = input
                    .try_parse(|input| input.expect_ident_matching("slashed-zero"))
                    .is_ok();
                if result.slashed_zero {
                    continue;
                }
            }
            break;
        }
        Ok(result)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for FontVariantNumeric {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        if let Some(value) = &self.numeric_figure_values {
            value.write_value(dest)?;
        }
        if let Some(value) = &self.numeric_spacing_values {
            value.write_value(dest)?;
        }
        if let Some(value) = &self.numeric_fraction_values {
            value.write_value(dest)?;
        }
        if self.ordinal {
            dest.write_str("ordinal")?;
        }
        if self.slashed_zero {
            dest.write_str("slashed-zero")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
/// Affects the amount that the current text position advances as each glyph is rendered
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/text.html#GlyphOrientationVerticalProperty)
pub enum GlyphOrientationVertical {
    #[default]
    /// Orientation set based on font
    Auto,
    /// An angle to be rounded to the closest 90deg interval
    Angle(Angle),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for GlyphOrientationVertical {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| input.expect_ident_matching("auto").map(|()| Self::Auto))
            .or_else(|_| Angle::parse(input).map(Self::Angle))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for GlyphOrientationVertical {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::Angle(angle) => angle.write_value(dest),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
/// Indicates that the user agent should adjust inter-glyph spacing
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/text.html#KerningProperty)
pub enum Kerning {
    #[default]
    /// Indicates that the user agent should adjust inter-glyph spacing.
    Auto,
    /// Auto-kerning is disabled. Instead, inter-character spacing is set to the given length.
    Length(Length),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Kerning {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| input.expect_ident_matching("auto").map(|()| Self::Auto))
            .or_else(|_| Length::parse(input).map(Self::Length))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Kerning {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::Length(length) => length.write_value(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A Length-percentage value
///
/// NOTE: SVG 1.1 specifies unitless px values, which is deprecated in SVG 2
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html#DataTypeLength)
/// [w3 | SVG 2](https://www.w3.org/TR/css-values/#typedef-length-percentage)
pub struct LengthPercentage(pub lightningcss::values::length::LengthPercentage);
impl LengthPercentage {
    /// Constructs a [`LengthPercentage`] with the given pixel value.
    pub fn px(val: f32) -> Self {
        Self(lightningcss::values::length::LengthPercentage::px(val))
    }
    #[allow(non_snake_case)]
    /// A percentage
    pub fn Percentage(percentage: Percentage) -> Self {
        Self(lightningcss::values::length::LengthPercentage::Percentage(
            percentage,
        ))
    }
    #[allow(non_snake_case)]
    /// A length
    pub fn Length(length: LengthValue) -> Self {
        Self(lightningcss::values::length::LengthPercentage::Dimension(
            length,
        ))
    }
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for LengthPercentage {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        lightningcss::values::length::LengthPercentage::parse(input).map(Self)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for LengthPercentage {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        if let Self(lightningcss::values::length::LengthPercentage::Dimension(LengthValue::Px(
            px,
        ))) = self
        {
            // NOTE: We're omitting length-value unit, since this is allowed in SVG 1.1
            px.write_value(dest)
        } else {
            self.0.write_value(dest)
        }
    }
}
impl Deref for LengthPercentage {
    type Target = lightningcss::values::length::LengthPercentage;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for LengthPercentage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A reference to an image or `<mask>` element to hide portions of an element.
///
/// [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/masking.html#MaskProperty)
/// [SVG 2](https://drafts.fxtf.org/css-masking-1/#propdef-mask)
pub struct Mask<'input>(pub ListOf<lightningcss::properties::masking::Mask<'input>, Comma>);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Mask<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        ListOf::parse(input).map(Self)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Mask<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_value(dest)
    }
}

enum_attr!(
    /// Enables `lnum` or `onum`
    NumericFigureValues {
        /// Enables `lnum`
        LiningNums: "lining-nums",
        /// Enables `onum`
        OldstyleNums: "oldstyle-nums",
    }
);
enum_attr!(
    /// Enables `pnum` or `tnum`
    NumericSpacingValues {
        /// Enables `pnum`
        ProportionalNums: "proportional-nums",
        /// Enables `tnum`
        TabularNums: "tabular-nums",
    }
);
enum_attr!(
    /// Enables `frac` and `afrc`
    NumericFractionValues {
        /// Enables `frac`
        DiagonalFractions: "diagonal-fractions",
        /// Enables `afrc`
        StackedFractions: "stacked-fractions",
    }
);

#[derive(Clone, Debug, Default, PartialEq)]
/// Enables/disables various open-type east-asian features.
///
/// [w3](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant)
pub struct FontVariantEastAsian {
    /// East Asian variant values
    east_asian_variant_values: Option<EastAsianVariantValues>,
    /// East Asian width values
    east_asian_width_values: Option<EastAsianWidthValues>,
    /// Enables `ruby`
    ruby: bool,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for FontVariantEastAsian {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let mut result = Self {
            east_asian_variant_values: None,
            east_asian_width_values: None,
            ruby: false,
        };
        loop {
            if result.east_asian_variant_values.is_none() {
                if let Ok(value) = input.try_parse(EastAsianVariantValues::parse) {
                    result.east_asian_variant_values = Some(value);
                    continue;
                }
            }
            if result.east_asian_width_values.is_none() {
                if let Ok(value) = input.try_parse(EastAsianWidthValues::parse) {
                    result.east_asian_width_values = Some(value);
                    continue;
                }
            }
            if !result.ruby {
                result.ruby = input
                    .try_parse(|input| input.expect_ident_matching("ruby"))
                    .is_ok();
                if result.ruby {
                    continue;
                }
            }
            break;
        }
        Ok(result)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for FontVariantEastAsian {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        if let Some(value) = &self.east_asian_variant_values {
            value.write_value(dest)?;
        }
        if let Some(value) = &self.east_asian_width_values {
            value.write_value(dest)?;
        }
        if self.ruby {
            dest.write_str("ruby")?;
        }
        Ok(())
    }
}
enum_attr!(
    /// Enables Japanese, Simplified, and Traditional variants
    EastAsianVariantValues {
        /// Enables `jp78`
        Jis78: "jis78",
        /// Enables `jp83`
        Jis83: "jis83",
        /// Enables `jp90`
        Jis90: "jis90",
        /// Enables `jp04`
        Jis04: "jis04",
        /// Enables `smpl`
        Simplified: "simplified",
        /// Enables `trad`
        Traditional: "traditional" ,
    }
);
enum_attr!(
    /// Enables `fwid` and `pwid`
    EastAsianWidthValues {
        /// Enables `fwid`
        FullWidth: "full-width",
        /// Enables `pwid`
        ProportionalWidth: "proportional-width",
    }
);

enum_attr!(
    /// Enables/disables various open-type sub/sup features.
    ///
    /// [w3](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant)
    FontVariantPosition {
        /// Enables `subs`
        Sub: "sub",
        /// Enables `sups`
        Super: "super",
    }
);

enum_attr!(
    /// Controls method of variant selection
    FontVariantEmoji {
        /// Renders emoji with unicode selector (`U+FE0E`)
        Text: "text",
        /// Renders emoji with emoji selector (`U+FE0F`)
        Emoji: "emoji",
        /// Overrides selector with emoji presentation properties
        Unicode: "unicode",
    }
);

enum_attr!(
    /// A paint operation
    Paint {
        /// The stroke operation
        Stroke: "stroke",
        /// The fill operation
        Fill: "fill",
        /// The markers operation
        Markers: "markers",
    }
);

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
#[cfg(feature = "parse")]
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
#[cfg(feature = "serialize")]
impl ToValue for PaintOrder {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        assert_eq!(self.0.len(), 3);
        if self.is_normal() {
            return dest.write_str("normal");
        }
        self.0[0].write_value(dest)?;
        dest.write_char(' ')?;
        self.0[1].write_value(dest)?;
        dest.write_char(' ')?;
        self.0[2].write_value(dest)
    }
}

/// A css position value
pub type Position = lightningcss::values::position::Position;

enum_attr!(
    /// Controls when the element can be the target of pointer events.
    ///
    /// [w3](https://svgwg.org/svg2-draft/interact.html#PointerEventsProperty)
    PointerEvents {
        /// The default behaviour, identical to `"visiblePainted"`.
        Auto: "auto",
        /// Only triggered when the element is visible and the pointer is over the bounding box of the element
        BoundingBox: "bounding-box",
        /// Only triggered when the element is visible and the pointer is on a painted area
        VisiblePainted: "visiblePainted",
        /// Only triggered when the element is visible and the pointer is on the interior (fill) area
        VisibleFill: "visibleFill",
        /// Only triggered when the element is visible and the pointer is on the perimiter (stroke) area
        VisibleStroke: "visibleStroke",
        /// Only triggered when the element is visible and the pointer is on the fill/stroke
        Visible: "visible",
        /// Only triggered when the pointer is on a painted area
        Painted: "painted",
        /// Only triggered when the pointer is on the interior (fill) area
        Fill: "fill",
        /// Only triggered when the pointer is on the perimiter (stroke) area
        Stroke: "stroke",
        /// Triggered when the pointer is on the fill/stroke
        All: "all",
        /// Never triggers
        None: "none",
    }
);
enum_attr!(
    /// Aligns text to a specific point based on the text-direction
    ///
    /// [w3](https://svgwg.org/svg2-draft/text.html#TextAnchorProperty)
    TextAnchor {
        /// Text is aligned to the initial position
        Start: "start",
        /// Text is aligned to the geometric middle
        Middle: "middle",
        /// Text is aligned to end of the rendered text
        End: "end",
    }
);

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

enum_attr!(
    /// Determines the direction in which lines are stacked.
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/text.html#WritingModeProperty)
    /// [w3 | CSS/SVG 2](https://drafts.csswg.org/css-writing-modes/#block-flow)
    WritingMode {
        /// Left-to-right, Top-to-bottom
        ///
        /// Equivalent to "horizontal-tb" in SVG 2
        LrTb: "lr-tb",
        /// Right-to-left, Top-to-bottom
        ///
        /// Equivalent to "horizontal-tb" in SVG 2
        RlTb: "rl-tb",
        /// Top-to-bottom, Right-to-left
        ///
        /// Equivalent to "vertical-rl" in SVG 2
        TbRl: "tb-rl",
        /// Left-to-right
        ///
        /// Equivalent to "horizontal-tb" in SVG 2
        Lr: "lr",
        /// Right-to-left
        ///
        /// Equivalent to "horizontal-tb" in SVG 2
        Rl: "rl",
        /// Top-to-bottom
        ///
        /// Equivalent to "vertical-rl" in SVG 2
        Tb: "tb",
        /// Top-to-bottom with horizontal writing/typograhy
        HorizontalTb: "horizontal-tb",
        /// Right-to-left with vertical writing/typography
        VerticalRl: "vertical-rl",
        /// Left-to-right with vertical writing/typography
        VerticalLr: "vertical-lr",
        /// Right-to-left with vertical writing and horizontal typography
        SidewaysRl: "sideways-rl",
        /// Left-to-right with vertical writing and horizontal typography
        SidewaysLr: "sideways-lr",
    }
);
