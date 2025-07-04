//! Style and presentation attribute types.
use cssparser_lightningcss::{match_ignore_ascii_case, ParseError, Parser, ParserInput, Token};
use itertools::Itertools;
use lightningcss::{
    declaration,
    error::{ParserError, PrinterError},
    printer::{self, Printer, PrinterOptions},
    properties::{
        self,
        custom::{CustomProperty, CustomPropertyName, TokenList},
        display::{Display, Visibility},
        effects::FilterList,
        font::{Font, FontFamily, FontSize, FontStretch, FontStyle, FontWeight},
        masking::{ClipPath, Mask},
        overflow::{Overflow, TextOverflow},
        position::Position,
        svg::{
            ColorInterpolation, ImageRendering, Marker, SVGPaint, ShapeRendering, StrokeDasharray,
            StrokeLinecap, StrokeLinejoin, TextRendering,
        },
        text::{Direction, Spacing, TextDecoration, UnicodeBidi},
        transform::{Matrix, Matrix3d, Transform, TransformList},
        ui::Cursor,
        Property, PropertyId,
    },
    rules::{self},
    stylesheet::{self, ParserOptions, StyleSheet},
    traits::{Parse, ParseWithOptions, ToCss, Zero},
    values::{
        alpha::AlphaValue,
        angle::Angle,
        color::CssColor,
        ident::Ident,
        length::LengthPercentage,
        number::CSSNumber,
        percentage::{DimensionPercentage, NumberOrPercentage},
        shape::FillRule,
    },
    vendor_prefix::VendorPrefix,
};
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use crate::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node,
    selectors::{SelectElement, Selector},
};

#[derive(Clone, PartialEq, Debug)]
/// An unparsed presentation attribute
pub struct UnparsedPresentationAttr<'i> {
    /// The presentation attribute's ID.
    pub presentation_attr_id: PresentationAttrId<'i>,
    /// The value of the attribute.
    pub value: TokenList<'i>,
}

impl<'i> UnparsedPresentationAttr<'i> {
    /// Parses a presentation attribute with the given id as a token list.
    ///
    /// # Errors
    /// If the custom-property name cannot be parsed
    pub fn parse<'t>(
        presentation_attr_id: PresentationAttrId<'i>,
        input: &mut Parser<'i, 't>,
        options: &ParserOptions<'_, 'i>,
    ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let value = CustomProperty::parse(CustomPropertyName::Unknown("".into()), input, options)?;
        input.expect_exhausted()?; // !important is not supported in presentation attributes
                                   // TODO: Port to lightningcss as
                                   // let value = input.parse_entirely(|input| TokenList::parse(input, options, 0))?;
        Ok(UnparsedPresentationAttr {
            presentation_attr_id,
            value: value.value,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
/// An unknown presentation attribute
pub struct UnknownPresentationAttr<'i> {
    /// The name of the attribute.
    pub name: Ident<'i>,
    /// The value of the attribute.
    pub value: TokenList<'i>,
}

impl<'i> UnknownPresentationAttr<'i> {
    fn parse<'t>(
        name: Ident<'i>,
        input: &mut Parser<'i, 't>,
        options: &ParserOptions<'_, 'i>,
    ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let value = CustomProperty::parse(CustomPropertyName::Unknown("".into()), input, options)?;
        // TODO: Port to lightningcss as
        // let value = input.parse_entirely(|input| TokenList::parse(input, options, 0))?;
        Ok(UnknownPresentationAttr {
            name,
            value: value.value,
        })
    }
}

macro_rules! define_presentation_attrs {
    (
        $(
            $name:literal: $attr:ident($type:ty $(, $vp:ty)?) $((inheritable: $inherit:ident))? $(/ $is_matching_property_id:ident)?,
        )+
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        /// A presentation attribute ID.
        pub enum PresentationAttrId<'i> {
            $(
                #[doc=concat!("The `", $name, "` attribute.")]
                $attr,
            )+
            /// An unknown or non-presentation attribute.
            Unknown(lightningcss::values::ident::Ident<'i>)
        }

        impl<'i> PresentationAttrId<'i> {
            /// Gets the name of the property as a string
            pub fn name(&self) -> &str {
                match self {
                    $(
                        PresentationAttrId::$attr => $name,
                    )+
                    PresentationAttrId::Unknown(name) => name.as_ref(),
                }
            }

            /// Whether a style is also applied to the element's children
            pub fn inheritable(&self) -> bool {
                macro_rules! inherit {
                    ($_inherit:ident) => { $_inherit };
                    () => { false };
                }
                match self {
                    $(
                        PresentationAttrId::$attr => inherit!($($inherit)?),
                    )+
                    _ => false,
                }
            }
        }

        impl<'i> From<&'i str> for PresentationAttrId<'i> {
            fn from(name: &'i str) -> PresentationAttrId<'i> {
                match name {
                    $(
                        $name => PresentationAttrId::$attr,
                    )+
                    _ => PresentationAttrId::Unknown(name.into()),
                }
            }
        }

        impl<'i> TryFrom<&PropertyId<'i>> for PresentationAttrId<'i> {
            type Error = ();

            fn try_from(value: &PropertyId<'i>) -> Result<PresentationAttrId<'i>, ()> {
                macro_rules! property_id_filtered {
                    ($_attr:ident) => { PropertyId::$_attr };
                    ($_attr:ident ($_vp:ty)) => { PropertyId::$_attr (_) };
                    ($_attr:ident / false $_name:literal) => { PropertyId::Custom(CustomPropertyName::Unknown(_)) };
                }
                macro_rules! property_id_if {
                    () => { true };
                    (false $_name:literal) => { value == &PropertyId::Custom(CustomPropertyName::Unknown($_name.into())) };
                }
                match value {
                    $(
                        property_id_filtered!($attr $(($vp))? $(/ $is_matching_property_id $name)?) if property_id_if!($($is_matching_property_id $name)?) => Ok(PresentationAttrId::$attr),
                    )+
                    _ => Err(()),
                }
            }
        }

        impl<'i> TryInto<PropertyId<'i>> for &PresentationAttrId<'i> {
            type Error = ();

            fn try_into(self) -> Result<PropertyId<'i>, ()> {
                macro_rules! try_property_id {
                    ($_attr:ident) => { Ok(PropertyId::$_attr) };
                    ($_attr:ident($_vp:ty)) => { Ok(PropertyId::$_attr(<$_vp>::None)) };
                    ($_attr:ident / false $_name:literal) => { match PropertyId::from($_name) {
                        PropertyId::Custom(_) => Err(()),
                        id => Ok(id),
                    } };
                }
                match self {
                    $(
                        PresentationAttrId::$attr => try_property_id!($attr $(($vp))? $(/ $is_matching_property_id $name)?),
                    )+
                    PresentationAttrId::Unknown(_) => Err(()),
                }
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        /// A presentation attribute
        pub enum PresentationAttr<'i> {
            $(
                #[doc=concat!("The `", $name, "` attribute.")]
                $attr($type),
            )+
            /// An unparsed presentation attribute.
            Unparsed(UnparsedPresentationAttr<'i>),
            /// An unknown or non-presentation attribute.
            Unknown(UnknownPresentationAttr<'i>),
        }

        impl<'i> PresentationAttr<'i> {
            /// Parses a presentation attribute by name.
            ///
            /// # Errors
            /// If the attribute cannot be parsed
            pub fn parse<'t>(presentation_attr_id: PresentationAttrId<'i>, input: &mut Parser<'i, 't>, options: &ParserOptions<'_, 'i>) -> Result<PresentationAttr<'i>, ParseError<'i, ParserError<'i>>> {
                let state = input.state();

                match presentation_attr_id {
                    $(
                        PresentationAttrId::$attr => {
                            if let Ok(c) = <$type>::parse_with_options(input, options) {
                                if input.expect_exhausted().is_ok() {
                                    return Ok(PresentationAttr::$attr(c));
                                }
                            }
                        },
                    )+
                    PresentationAttrId::Unknown(name) => return Ok(PresentationAttr::Unknown(UnknownPresentationAttr::parse(name, input, options)?)),
                };

                input.reset(&state);
                return Ok(PresentationAttr::Unparsed(UnparsedPresentationAttr::parse(presentation_attr_id, input, options)?))
            }

            /// Returns the presentation attribute's id for this presentation attribute.
            pub fn presentation_attr_id(&self) -> PresentationAttrId<'i> {
                use PresentationAttr::*;

                match self {
                    $(
                        $attr(_) => PresentationAttrId::$attr,
                    )+
                    Unparsed(unparsed) => unparsed.presentation_attr_id.clone(),
                    Unknown(unknown) => PresentationAttrId::Unknown(unknown.name.clone())
                }
            }

            /// Parses a presentation attribute from a string.
            ///
            /// # Errors
            /// If the string cannot be parsed.
            pub fn parse_string(presentation_attr_id: PresentationAttrId<'i>, input: &'i str, options: ParserOptions<'_, 'i>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
                let mut input = ParserInput::new(input);
                let mut parser = Parser::new(&mut input);
                Self::parse(presentation_attr_id, &mut parser, &options)
            }

            /// Serializes the presntation attribute without it's name.
            ///
            /// # Errors
            /// If the value cannot be serialized
            pub fn value_to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError> where W: std::fmt::Write {
                // TODO: After porting, to `to_css` on `TokenList` directly
                match self {
                    $(
                        PresentationAttr::$attr(val) => val.to_css(dest),
                    )+
                    PresentationAttr::Unknown(unknown) => {
                      Property::Custom(CustomProperty {
                              name: CustomPropertyName::Unknown("".into()),
                              value: unknown.value.clone()
                          }).value_to_css(dest)
                    },
                    PresentationAttr::Unparsed(unparsed) => Property::Custom(CustomProperty {
                            name: CustomPropertyName::Unknown("".into()),
                            value: unparsed.value.clone(),
                        }).value_to_css(dest),
                }
            }

            /// Serializes the presentation attribute as a string.
            ///
            /// # Errors
            /// If the value cannot be serialized
            pub fn value_to_css_string(&self, options: PrinterOptions) -> Result<String, PrinterError> {
                let mut s = String::new();
                let mut printer = Printer::new(&mut s, options);
                self.value_to_css(&mut printer)?;
                Ok(s)
            }
        }

        impl<'i> ToCss for PresentationAttr<'i> {
            fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError> where W: std::fmt::Write {
                let name = match self {
                    $(
                        PresentationAttr::$attr(_) => $name,
                    )+
                    PresentationAttr::Unknown(unknown) => unknown.name.as_ref(),
                    PresentationAttr::Unparsed(unparsed) => unparsed.presentation_attr_id.name(),
                };
                dest.write_str(name)?;
                dest.write_char('=')?;
                self.value_to_css(dest)
            }

        }
    };
}

define_presentation_attrs! {
    "alignment-baseline": AlignmentBaseline(AlignmentBaseline) / false,
    "baseline-shift": BaselineShift(BaselineShift) / false,
    "clip-path": ClipPath(ClipPath<'i>, VendorPrefix),
    "clip-rule": ClipRule(FillRule) (inheritable: true),
    // DEPRECATED: "clip"
    "color-interpolation-filters": ColorInterpolationFilters(ColorInterpolation) (inheritable: true),
    "color-interpolation": ColorInterpolation(ColorInterpolation) (inheritable: true),
    // DEPRECATED: "color-profile"
    // OBSOLETE: "color-rendering"
    "color": Color(CssColor) (inheritable: true),
    "cursor": Cursor(Cursor<'i>) (inheritable: true),
    "direction": Direction(Direction) (inheritable: true),
    "display": Display(Display),
    "dominant-baseline": DominantBaseline(DominantBaseline) (inheritable: true) / false,
    // DEPRECATED: "enable-background"
    "fill-opacity": FillOpacity(AlphaValue) (inheritable: true),
    "fill-rule": FillRule(FillRule) (inheritable: true),
    "fill": Fill(SVGPaint<'i>) (inheritable: true),
    "filter": Filter(FilterList<'i>, VendorPrefix),
    "flood-color": FloodColor(CssColor) / false,
    "flood-opacity": FloodOpacity(AlphaValue) / false,
    "font-family": FontFamily(Vec<FontFamily<'i>>) (inheritable: true),
    "font-size-adjust": FontSizeAdjust(CSSNumber) (inheritable: true) / false,
    "font-size": FontSize(FontSize) (inheritable: true),
    "font-stretch": FontStretch(FontStretch) (inheritable: true),
    "font-style": FontStyle(FontStyle) (inheritable: true),
    "font-variant": FontVariant(FontVariant<'i>) (inheritable: true) / false,
    "font-weight": FontWeight(FontWeight) (inheritable: true),
    "font": Font(Font<'i>),
    // DEPRECATED: "glyph-orientation-horizontal"
    // DEPRECATED: "glyph-orientation-vertical"
    "image-rendering": ImageRendering(ImageRendering) (inheritable: true),
    "letter-spacing": LetterSpacing(Spacing) (inheritable: true),
    "lighting-color": LightingColor(CssColor) / false,
    "marker-end": MarkerEnd(Marker<'i>) (inheritable: true),
    "marker-mid": MarkerMid(Marker<'i>) (inheritable: true),
    "marker-start": MarkerStart(Marker<'i>) (inheritable: true),
    "marker": Marker(Marker<'i>) (inheritable: true),
    "mask": Mask(SmallVec<[Mask<'i>; 1]>, VendorPrefix),
    "opacity": Opacity(AlphaValue),
    "overflow": Overflow(Overflow),
    "paint-order": PaintOrder(PaintOrder) (inheritable: true) / false,
    "pointer-events": PointerEvents(PointerEvents) (inheritable: true) / false,
    "shape-rendering": ShapeRendering(ShapeRendering) (inheritable: true),
    "stop-color": StopColor(CssColor) / false,
    "stop-opacity": StopOpacity(AlphaValue) / false,
    "stroke-dasharray": StrokeDasharray(StrokeDasharray) (inheritable: true),
    "stroke-dashoffset": StrokeDashoffset(LengthPercentage) (inheritable: true),
    "stroke-linecap": StrokeLinecap(StrokeLinecap) (inheritable: true),
    "stroke-linejoin": StrokeLinejoin(StrokeLinejoin) (inheritable: true),
    "stroke-miterlimit": StrokeMiterlimit(CSSNumber) (inheritable: true),
    "stroke-opacity": StrokeOpacity(AlphaValue) (inheritable: true),
    "stroke-width": StrokeWidth(LengthPercentage) (inheritable: true),
    "stroke": Stroke(SVGPaint<'i>) (inheritable: true),
    "text-anchor": TextAnchor(TextAnchor) (inheritable: true) / false,
    "text-decoration": TextDecoration(TextDecoration, VendorPrefix),
    "text-overflow": TextOverflow(TextOverflow, VendorPrefix),
    "text-rendering": TextRendering(TextRendering) (inheritable: true),
    "transform-origin": TransformOrigin(Position, VendorPrefix),
    "transform": Transform(SVGTransformList, VendorPrefix) (inheritable: true),
    "unicode-bidi": UnicodeBidi(UnicodeBidi),
    "vector-effect": VectorEffect(VectorEffect) / false,
    "visibility": Visibility(Visibility) (inheritable: true),
    "word-spacing": WordSpacing(Spacing) (inheritable: true),
    "writing-mode": WritingMode(WritingMode) (inheritable: true) / false,

    // NOTE: I think these may technically may not be presentation attrs, but they contain css so
    // may be worth considering
    // TODO: If so, maybe `is_presentation` method would be needed
    "gradientTransform": GradientTransform(SVGTransformList) / false,
    "patternTransform": PatternTransform(SVGTransformList) / false,

    // NOTE: We could include `d`, as is done with https://github.com/parcel-bundler/lightningcss/pull/868
    // "d": Definition(PathData) / false,
}

// TODO: Port all these to `lighningscss::properties::presentation`
// TODO: Use `enum_propery!` after porting where possible
#[derive(Debug, PartialEq, Clone)]
/// The `alignment-baseline` attribute specifies how an object is aligned with respect to its parent.
pub enum AlignmentBaseline {
    /// The value is the dominant-baseline of the script to which the character belongs - i.e., use the dominant-baseline of the parent.
    Auto,
    /// Uses the dominant baseline choice of the parent. Matches the box's corresponding [baseline](https://developer.mozilla.org/en-US/docs/Glossary/Baseline/Typography) to that of its parent.
    Baseline,
    /// The alignment-point of the object being aligned is aligned with the "before-edge" baseline of the parent text content element.
    BeforeEdge,
    /// Matches the bottom of the box to the top of the parent's content area.
    TextBottom,
    /// The alignment-point of the object being aligned is aligned with the "text-before-edge" baseline of the parent text content element.
    ///
    /// > [!NOTE] This keyword may be mapped to `text-top`.
    TextBeforeEdge,
    /// Aligns the vertical midpoint of the box with the baseline of the parent box plus half the x-height of the parent.
    Middle,
    /// Matches the box's central baseline to the central baseline of its parent.
    Central,
    /// The alignment-point of the object being aligned is aligned with the "after-edge" baseline of the parent text content element.
    AfterEdge,
    /// Matches the top of the box to the top of the parent's content area.
    TextTop,
    /// The alignment-point of the object being aligned is aligned with the "text-after-edge" baseline of the parent text content element.
    ///
    /// > [!NOTE] This keyword may be mapped to `text-bottom`.
    TextAfterEdge,
    /// Matches the box's ideographic character face under-side baseline to that of its parent.
    Ideographic,
    /// Matches the box's alphabetic baseline to that of its parent.
    Alphabetic,
    /// The alignment-point of the object being aligned is aligned with the "hanging" baseline of the parent text content element.
    Hanging,
    /// Matches the box's mathematical baseline to that of its parent.
    Mathematical,
    /// Aligns the top of the aligned subtree with the top of the line box.
    Top,
    /// Aligns the center of the aligned subtree with the center of the line box.
    Center,
    /// Aligns the bottom of the aligned subtree with the bottom of the line box.
    Bottom,
}

impl<'i> Parse<'i> for AlignmentBaseline {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "auto" => Ok(AlignmentBaseline::Auto),
            "baseline" => Ok(AlignmentBaseline::Baseline),
            "before-edge" => Ok(AlignmentBaseline::BeforeEdge),
            "text-bottom" => Ok(AlignmentBaseline::TextBottom),
            "text-before-edge" => Ok(AlignmentBaseline::TextBeforeEdge),
            "middle" => Ok(AlignmentBaseline::Middle),
            "central" => Ok(AlignmentBaseline::Central),
            "after-edge" => Ok(AlignmentBaseline::AfterEdge),
            "text-top" => Ok(AlignmentBaseline::TextTop),
            "text-after-edge" => Ok(AlignmentBaseline::TextAfterEdge),
            "ideographic" => Ok(AlignmentBaseline::Ideographic),
            "alphabetic" => Ok(AlignmentBaseline::Alphabetic),
            "hanging" => Ok(AlignmentBaseline::Hanging),
            "mathematical" => Ok(AlignmentBaseline::Mathematical),
            "top" => Ok(AlignmentBaseline::Top),
            "center" => Ok(AlignmentBaseline::Center),
            "bottom" => Ok(AlignmentBaseline::Bottom),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for AlignmentBaseline {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let ident = match self {
            AlignmentBaseline::Auto => "auto",
            AlignmentBaseline::Baseline => "baseline",
            AlignmentBaseline::BeforeEdge => "before-edge",
            AlignmentBaseline::TextBottom | AlignmentBaseline::TextAfterEdge => "text-bottom",
            AlignmentBaseline::Middle => "middle",
            AlignmentBaseline::Central => "central",
            AlignmentBaseline::AfterEdge => "after-edge",
            AlignmentBaseline::TextTop | AlignmentBaseline::TextBeforeEdge => "text-top",
            AlignmentBaseline::Ideographic => "ideographic",
            AlignmentBaseline::Alphabetic => "alphabetic",
            AlignmentBaseline::Hanging => "hanging",
            AlignmentBaseline::Mathematical => "mathematical",
            AlignmentBaseline::Top => "top",
            AlignmentBaseline::Center => "center",
            AlignmentBaseline::Bottom => "bottom",
        };
        dest.write_str(ident)
    }
}

/// The baseline-shift attribute allows repositioning of the dominant-baseline relative to the dominant-baseline of the parent text content element.
#[derive(Debug, Clone, PartialEq)]
pub enum BaselineShift {
    /// A length value raises (positive value) or lowers (negative value) the dominant-baseline of the parent text content element by the specified length.
    ///
    /// A percentage value raises (positive value) or lowers (negative value) the dominant-baseline of the parent text content element by the specified percentage of the `line-height`.
    LengthPercentage(LengthPercentage),
    /// The dominant-baseline is shifted to the default position for subscripts.
    Sub,
    /// The dominant-baseline is shifted to the default position for superscripts.
    Super,
}

impl<'i> Parse<'i> for BaselineShift {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        if let Ok(val) = input.try_parse(LengthPercentage::parse) {
            return Ok(BaselineShift::LengthPercentage(val));
        }

        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "sub" => Ok(BaselineShift::Sub),
            "super" => Ok(BaselineShift::Super),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for BaselineShift {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            BaselineShift::LengthPercentage(val) => val.to_css(dest),
            BaselineShift::Sub => dest.write_str("sub"),
            BaselineShift::Super => dest.write_str("super"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// The `dominant-baseline` attribute specifies the dominant baseline, which is the baseline used to align the box's text and inline-level contents.
///
/// [MDN | dominant-baseline](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/dominant-baseline)
pub enum DominantBaseline {
    /// The computed value depends on the `writing-mode` attribute
    Auto,
    /// Uses the bottom of the box
    TextBottom,
    /// Uses the alphabetic baseline
    Alphabetic,
    /// Uses the ideographic baseline
    Ideographic,
    /// Uses the middle baseline
    Middle,
    /// Uses the central baseline
    Central,
    /// Uses the mathematical baseline
    Mathematical,
    /// Uses the hanging baseline
    Hanging,
    /// Uses the top of the box
    TextTop,
}

impl<'i> Parse<'i> for DominantBaseline {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "auto" => Ok(DominantBaseline::Auto),
            "text-bottom" => Ok(DominantBaseline::TextBottom),
            "alphabetic" => Ok(DominantBaseline::Alphabetic),
            "ideographic" => Ok(DominantBaseline::Ideographic),
            "middle" => Ok(DominantBaseline::Middle),
            "central" => Ok(DominantBaseline::Central),
            "mathematical" => Ok(DominantBaseline::Mathematical),
            "hanging" => Ok(DominantBaseline::Hanging),
            "text-top" => Ok(DominantBaseline::TextTop),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for DominantBaseline {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let ident = match self {
            DominantBaseline::Auto => "auto",
            DominantBaseline::TextBottom => "text-bottom",
            DominantBaseline::Alphabetic => "alphabetic",
            DominantBaseline::Ideographic => "ideographic",
            DominantBaseline::Middle => "middle",
            DominantBaseline::Central => "central",
            DominantBaseline::Mathematical => "mathematical",
            DominantBaseline::Hanging => "hanging",
            DominantBaseline::TextTop => "text-top",
        };
        dest.write_str(ident)
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Indicates whether the text is to be rendered using variations of the font's glyphs
///
/// [MDN | font-variant](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/font-variant)
pub struct FontVariant<'i>(TokenList<'i>);

impl<'i> Parse<'i> for FontVariant<'i> {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        Ok(FontVariant(TokenList::parse_with_options(
            input,
            &ParserOptions::default(),
        )?))
    }
}

impl ToCss for FontVariant<'_> {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        Property::Custom(CustomProperty {
            name: CustomPropertyName::Unknown("".into()),
            value: self.0.clone(),
        })
        .value_to_css(dest)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
enum Paint {
    Stroke,
    Fill,
    Markers,
}

impl<'i> Parse<'i> for Paint {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "stroke" => Ok(Paint::Stroke),
            "fill" => Ok(Paint::Fill),
            "markers" => Ok(Paint::Markers),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for Paint {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let ident = match self {
            Paint::Stroke => "stroke",
            Paint::Fill => "fill",
            Paint::Markers => "markers",
        };
        dest.write_str(ident)
    }
}

#[derive(Debug, Hash, PartialEq, Clone)]
/// Controls the order in which the steps of painting are done
///
/// [MDN | paint-order](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/paint-order)
pub struct PaintOrder(SmallVec<[Paint; 3]>);

impl<'i> Parse<'i> for PaintOrder {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let state = input.state();
        if let Ok(ident) = input.expect_ident() {
            match_ignore_ascii_case! { ident,
                "normal" => return Ok(Self::normal()),
                _ => {},
            };
        }
        input.reset(&state);
        let location = input.current_source_location();
        let output = input.try_parse(SmallVec::<[Paint; 3]>::parse)?;
        if output.len() > 3 {
            return Err(location.new_custom_error(ParserError::InvalidValue));
        }
        let unique: HashSet<_> = output.iter().collect();
        if unique.len() != output.len() {
            return Err(location.new_custom_error(ParserError::InvalidValue));
        }
        Ok(PaintOrder(output))
    }
}

impl ToCss for PaintOrder {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        if self == &Self::normal() {
            dest.write_str("normal")
        } else {
            self.0.to_css(dest)
        }
    }
}

impl PaintOrder {
    fn normal() -> Self {
        PaintOrder(smallvec![Paint::Fill, Paint::Stroke, Paint::Markers])
    }
}

#[derive(Debug, Hash, Clone, PartialEq)]
/// Determines whether or when an element is the target of a mouse event
///
/// [MDN | pointer-events](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/pointer-events)
pub enum PointerEvents {
    /// The element can only be the target if the mouse is over the element's bounding box
    BoundingBox,
    /// The element can only be the target if it's computed `visibility` style is `visible`
    /// and the mouse is over a painted `fill` or `stroke` portion of the element.
    ///
    /// This is equivalent to using `auto`
    VisiblePainted,
    /// The element can only be the target if it's computed `visibility` style is `visible`
    /// and the mouse is over a painted `fill` portion of the element.
    VisibleFill,
    /// The element can only be the target if it's computed `visibility` style is `visible`
    /// and the mouse is over a painted `stroke` portion of the element.
    VisibleStroke,
    /// The element can only be the target if it's computed `visibility`.
    Visible,
    /// The element can only the the target if the mouse is over a painted `fill` or `stroke`
    /// portion of the element.
    Painted,
    /// The element can only the the target if the mouse is over a painted `fill` portion of
    /// the element.
    Fill,
    /// The element can only the the target if the mouse is over a painted `stroke` portion
    /// of the element.
    Stroke,
    /// The element can be the target regardless of `visibility`, `fill`, or `stroke`
    All,
    /// The element can never be the target
    None,
}

impl<'i> Parse<'i> for PointerEvents {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "bounding-box" => Ok(PointerEvents::BoundingBox),
            "visible-painted" => Ok(PointerEvents::VisiblePainted),
            "visible-full" => Ok(PointerEvents::VisibleFill),
            "visible-stroke" => Ok(PointerEvents::VisibleStroke),
            "visible" => Ok(PointerEvents::Visible),
            "painted" => Ok(PointerEvents::Painted),
            "fill" => Ok(PointerEvents::Fill),
            "stroke" => Ok(PointerEvents::Stroke),
            "all" => Ok(PointerEvents::All),
            "none" => Ok(PointerEvents::None),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for PointerEvents {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let ident = match self {
            PointerEvents::BoundingBox => "bounding-box",
            PointerEvents::VisiblePainted => "visible-painted",
            PointerEvents::VisibleFill => "visible-full",
            PointerEvents::VisibleStroke => "visible-stroke",
            PointerEvents::Visible => "visible",
            PointerEvents::Painted => "painted",
            PointerEvents::Fill => "fill",
            PointerEvents::Stroke => "stroke",
            PointerEvents::All => "all",
            PointerEvents::None => "none",
        };
        dest.write_str(ident)
    }
}

#[derive(Debug, Hash, Clone, PartialEq)]
/// Used to align text.
///
/// [MDN | text-anchor](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/text-anchor)
pub enum TextAnchor {
    /// Render characters so that the start of text is rendered at the element's position.
    Start,
    /// Render characters so that the middle of the text is rendered at the element's position.
    Middle,
    /// Render characters so that the end of the text is rendered at the element's position.
    End,
}

impl<'i> Parse<'i> for TextAnchor {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "start" => Ok(TextAnchor::Start),
            "middle" => Ok(TextAnchor::Middle),
            "end" => Ok(TextAnchor::End),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for TextAnchor {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let ident = match self {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };
        dest.write_str(ident)
    }
}

#[derive(Debug)]
/// Precision options use for rounding different types
pub struct Precision {
    /// The precision to use when rounding translations
    pub float: i32,
    /// The precision to use when rounding degrees
    pub deg: i32,
    /// The precision to use when rounding transforms
    pub transform: i32,
}

impl Precision {
    /// Rounds a number to a given precision
    fn round_arg(precision: i32, data: &mut f32) {
        *data = if (1..20).contains(&precision) {
            Self::smart_round(precision, *data)
        } else {
            data.round()
        }
    }

    /// Rounds a number to a given precision
    fn smart_round(precision: i32, data: f32) -> f32 {
        let tolerance = Self::to_fixed(0.1_f32.powi(precision), precision);
        if Self::to_fixed(data, precision) == data {
            data
        } else {
            let rounded = Self::to_fixed(data, precision - 1);
            if Self::to_fixed((rounded - data).abs(), precision + 1) >= tolerance {
                Self::to_fixed(data, precision)
            } else {
                rounded
            }
        }
    }

    fn to_fixed(data: f32, precision: i32) -> f32 {
        let pow = 10.0_f32.powi(precision);
        f32::round(data * pow) / pow
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A list of transform definitions applied to an element and it's children.
///
/// [MDN | transform](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/transform)
pub struct SVGTransformList(pub Vec<SVGTransform>);

impl SVGTransformList {
    fn optimize(rounded: &[SVGTransform], raw: &[SVGTransform]) -> Vec<SVGTransform> {
        assert_eq!(rounded.len(), raw.len());

        let mut optimized = vec![];
        let mut skip = false;
        for i in 0..rounded.len() {
            if skip {
                skip = false;
                continue;
            }
            let item = &rounded[i];
            if item.is_identity() {
                continue;
            }
            match item {
                SVGTransform::Rotate(n, 0.0, 0.0) if n.abs() == 180.0 => {
                    if let Some(SVGTransform::Scale(x, y)) = rounded.get(i + 1) {
                        optimized.push(SVGTransform::Scale(-x, -y));
                        skip = true;
                    } else {
                        optimized.push(SVGTransform::Scale(-1.0, -1.0));
                    }
                }
                SVGTransform::Rotate(..) => {
                    optimized.push(item.clone());
                }
                SVGTransform::Translate(..) => {
                    if let Some(SVGTransform::Rotate(n, x, y)) = rounded.get(i + 1) {
                        if *n != 180.0 && *n != -180.0 && *n != 0.0 && *x == 0.0 && *y == 0.0 {
                            log::debug!("merging translate and rotate");
                            let translate = &raw[i];
                            let rotate = &raw[i + 1];
                            optimized
                                .push(SVGTransform::merge_translate_and_rotate(translate, rotate));
                            skip = true;
                            continue;
                        }
                    }
                    optimized.push(item.clone());
                }
                SVGTransform::Matrix(_) => unreachable!(),
                _ => optimized.push(item.clone()),
            }
        }

        if optimized.is_empty() {
            vec![SVGTransform::Scale(1.0, 1.0)]
        } else {
            optimized
        }
    }

    /// Converts the transform to a 3D matrix.
    pub fn to_matrix(&self) -> Option<Matrix3d<CSSNumber>> {
        let mut matrix = Matrix3d::identity();
        for transform in &self.0 {
            let transform = if let SVGTransform::Rotate(angle, 0.0, 0.0) = transform {
                Transform::Matrix3d(Matrix3d::rotate(0.0, 0.0, 1.0, angle.to_radians()))
            } else {
                transform.clone().into()
            };
            if let Some(m) = transform.to_matrix() {
                matrix = m.multiply(&matrix);
            } else {
                return None;
            }
        }
        Some(matrix)
    }

    /// Attempts to convert the matrix to 2D.
    pub fn to_matrix_2d(&self) -> Option<Matrix<CSSNumber>> {
        self.to_matrix().and_then(|m| m.to_matrix2d())
    }
}

impl From<SVGTransformList> for TransformList {
    fn from(val: SVGTransformList) -> Self {
        TransformList(val.0.into_iter().map(std::convert::Into::into).collect())
    }
}

impl TryFrom<&TransformList> for SVGTransformList {
    type Error = ();

    fn try_from(value: &TransformList) -> Result<Self, Self::Error> {
        let list: Result<Vec<_>, _> = value.0.iter().map(TryInto::try_into).collect();
        Ok(Self(list?))
    }
}

#[derive(Debug, Clone, PartialEq)]
/// A transform applied to an element and it's children
///
/// [MDN | transform](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/transform)
pub enum SVGTransform {
    /// A transformation as the matrix of six values
    Matrix(Matrix<f32>),
    /// A positional transformation in an `x` and/or `y` direction
    Translate(f32, f32),
    /// A size transformation in an `x` and/or `y` direction
    Scale(f32, f32),
    /// A rotational transform by `a` degrees around an `x` and `y` origin
    Rotate(f32, f32, f32),
    /// A skew transform in the `x` direction
    SkewX(f32),
    /// A skew transform in the `y` direction
    SkewY(f32),
    /// A transform provided in a CSS format
    CssTransform(Transform),
}

impl SVGTransform {
    /// Round the arguments of the transform to a given precision
    pub fn round(&mut self, precision: &Precision) {
        match self {
            SVGTransform::Translate(x, y) => {
                let precision = precision.float;
                Precision::round_arg(precision, x);
                Precision::round_arg(precision, y);
            }
            SVGTransform::Rotate(d, x, y) => {
                Precision::round_arg(precision.deg, d);
                let precision = precision.float;
                Precision::round_arg(precision, x);
                Precision::round_arg(precision, y);
            }
            SVGTransform::SkewX(a) | SVGTransform::SkewY(a) => {
                Precision::round_arg(precision.deg, a);
            }
            SVGTransform::Scale(x, y) => {
                let precision = precision.transform;
                Precision::round_arg(precision, x);
                Precision::round_arg(precision, y);
            }
            SVGTransform::Matrix(m) => {
                let p = precision.transform;
                Precision::round_arg(p, &mut m.a);
                Precision::round_arg(p, &mut m.b);
                Precision::round_arg(p, &mut m.c);
                Precision::round_arg(p, &mut m.d);
                let p = precision.float;
                Precision::round_arg(p, &mut m.e);
                Precision::round_arg(p, &mut m.f);
            }
            SVGTransform::CssTransform(_) => {}
        }
    }

    fn round_vec(transforms: &mut [Self], precision: &Precision) {
        transforms
            .iter_mut()
            .for_each(|transform| transform.round(precision));
    }

    /// Converts a matrix to the shortest form of a transform
    pub fn matrix_to_transform(&self, precision: &Precision) -> Vec<Self> {
        let mut shortest = vec![self.clone()];
        let Self::Matrix(m) = self else {
            return shortest;
        };

        let decomposed = Self::get_compositions(m);

        Self::round_vec(&mut shortest, precision);
        let Ok(starting_string) = shortest[0].to_css_string(PrinterOptions::default()) else {
            return vec![self.clone()];
        };
        let mut shortest_len = starting_string.len();
        log::debug!("converting matrix to transform: {starting_string} ({shortest_len})");
        for decomposition in decomposed {
            let mut rounded_transforms = decomposition.clone();
            Self::round_vec(&mut rounded_transforms, precision);

            let mut optimized = SVGTransformList::optimize(&rounded_transforms, &decomposition);
            Self::round_vec(&mut optimized, precision);
            let optimized_string = optimized
                .iter()
                .map(|item| {
                    item.to_css_string(PrinterOptions::default())
                        .unwrap_or_default()
                })
                .join("");
            log::debug!("optimized: {optimized_string} ({})", optimized_string.len());
            if optimized_string.len() <= shortest_len {
                shortest = optimized;
                shortest_len = optimized_string.len();
            }
        }

        log::debug!("converted to transform: {:?}", shortest);
        shortest
    }

    fn get_compositions(matrix: &Matrix<f32>) -> Vec<Vec<SVGTransform>> {
        let mut decompositions = vec![];

        if let Some(qrcd) = Self::qrcd(matrix) {
            log::debug!(r#"decomposed qrcd: "{:?}""#, qrcd);
            decompositions.push(qrcd);
        }
        if let Some(qrab) = Self::qrab(matrix) {
            log::debug!(r#"decomposed qrab: "{:?}""#, qrab);
            decompositions.push(qrab);
        }
        decompositions
    }

    fn qrab(matrix: &Matrix<f32>) -> Option<Vec<SVGTransform>> {
        let Matrix { a, b, c, d, e, f } = matrix;
        let delta = a * d - b * c;
        if delta == 0.0 {
            return None;
        }

        let radius = f32::hypot(*a, *b);
        if radius == 0.0 {
            return None;
        }

        let mut decomposition = vec![];
        let cos = a / radius;

        if *e != 0.0 || *f != 0.0 {
            decomposition.push(SVGTransform::Translate(*e, *f));
        }

        if cos != 1.0 {
            let mut rad = cos.acos();
            if *b < 0.0 {
                rad *= -1.0;
            }
            decomposition.push(SVGTransform::Rotate(rad.to_degrees(), 0.0, 0.0));
        }

        let sx = radius;
        let sy = delta / sx;
        if sx != 1.0 || sy != 1.0 {
            decomposition.push(SVGTransform::Scale(sx, sy));
        }

        let ac_bd = a * c + b * d;
        if ac_bd != 0.0 {
            decomposition.push(SVGTransform::SkewX(
                (ac_bd / (a * a + b * b)).atan().to_degrees(),
            ));
        }

        Some(decomposition)
    }

    fn qrcd(matrix: &Matrix<f32>) -> Option<Vec<SVGTransform>> {
        let Matrix { a, b, c, d, e, f } = matrix;

        let delta = a * d - b * c;
        if delta == 0.0 {
            return None;
        }
        let s = f32::hypot(*c, *d);
        if s == 0.0 {
            return None;
        }

        let mut decomposition = vec![];

        if *e != 0.0 || *f != 0.0 {
            decomposition.push(SVGTransform::Translate(*e, *f));
        }

        let rad =
            std::f32::consts::PI / 2.0 - (if *d < 0.0 { -1.0 } else { 1.0 }) * f32::acos(-c / s);
        decomposition.push(SVGTransform::Rotate(rad.to_degrees(), 0.0, 0.0));

        let sx = delta / s;
        let sy = s;
        if sx != 1.0 || sy != 1.0 {
            decomposition.push(SVGTransform::Scale(sx, sy));
        }

        let ac_bd = a * c + b * d;
        if ac_bd != 0.0 {
            decomposition.push(SVGTransform::SkewY(
                f32::atan(ac_bd / (c * c + d * d)).to_degrees(),
            ));
        }

        Some(decomposition)
    }

    fn is_identity(&self) -> bool {
        match self {
            Self::Rotate(n, _, _) | Self::SkewX(n) | Self::SkewY(n) => *n == 0.0,
            Self::Scale(x, y) | Self::Translate(x, y) => *x == 1.0 && *y == 1.0,
            Self::Matrix(_) | Self::CssTransform(_) => false,
        }
    }

    fn merge_translate_and_rotate(translate: &Self, rotate: &Self) -> Self {
        let Self::Translate(tx, ty) = translate else {
            unreachable!();
        };
        let Self::Rotate(a, 0.0, 0.0) = rotate else {
            unreachable!();
        };

        let rad = a.to_radians();
        let d = 1.0 - rad.cos();
        let e = rad.sin();
        let cy = (d * ty + e * tx) / (d * d + e * e);
        let cx = (tx - e * cy) / d;
        Self::Rotate(*a, cx, cy)
    }
}

impl<'i> Parse<'i> for SVGTransform {
    #[allow(clippy::many_single_char_names)]
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        input
            .try_parse(|input| {
                // SVG transforms allow whitespace between the function name and arguments, unlike CSS functions.
                // So this may tokenize either as a function, or as an ident followed by a parenthesis block.
                let function = input
                    .try_parse(|input| input.expect_function().cloned())
                    .or_else(|_| -> Result<_, ParseError<_>> {
                        let name = input.expect_ident_cloned()?;
                        input.skip_whitespace();
                        input.expect_parenthesis_block()?;
                        Ok(name)
                    })?;

                let function_case_sensitive: &str = &function;
                input.parse_nested_block(|input| {
                    let location = input.current_source_location();
                    match function_case_sensitive {
                        "matrix" => {
                            let a = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let b = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let c = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let d = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let e = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            let f = f32::parse(input)?;
                            Ok(SVGTransform::Matrix(Matrix { a, b, c, d, e, f }))
                        }
                        "translate" => {
                            let x = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            if let Ok(y) = input.try_parse(f32::parse) {
                                Ok(SVGTransform::Translate(x, y))
                            } else {
                                Ok(SVGTransform::Translate(x, 0.0))
                            }
                        }
                        "scale" => {
                            let x = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            if let Ok(y) = input.try_parse(f32::parse) {
                                Ok(SVGTransform::Scale(x, y))
                            } else {
                                Ok(SVGTransform::Scale(x, x))
                            }
                        }
                        "rotate" => {
                            let angle = f32::parse(input)?;
                            skip_comma_and_whitespace(input);
                            if let Ok(x) = input.try_parse(f32::parse) {
                                skip_comma_and_whitespace(input);
                                let y = f32::parse(input)?;
                                Ok(SVGTransform::Rotate(angle, x, y))
                            } else {
                                Ok(SVGTransform::Rotate(angle, 0.0, 0.0))
                            }
                        }
                        "skewX" => {
                            let angle = f32::parse(input)?;
                            Ok(SVGTransform::SkewX(angle))
                        }
                        "skewY" => {
                            let angle = f32::parse(input)?;
                            Ok(SVGTransform::SkewY(angle))
                        }
                        _ => {
                            Err(location.new_unexpected_token_error(Token::Ident(function.clone())))
                        }
                    }
                })
            })
            .or_else(|_| Ok(SVGTransform::CssTransform(Transform::parse(input)?)))
    }
}

fn skip_comma_and_whitespace(input: &mut Parser<'_, '_>) {
    input.skip_whitespace();
    let _ = input.try_parse(Parser::expect_comma);
    input.skip_whitespace();
}

impl ToCss for SVGTransform {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            SVGTransform::Matrix(Matrix { a, b, c, d, e, f }) => {
                dest.write_str("matrix(")?;
                a.to_css(dest)?;
                dest.write_char(' ')?;
                b.to_css(dest)?;
                dest.write_char(' ')?;
                c.to_css(dest)?;
                dest.write_char(' ')?;
                d.to_css(dest)?;
                dest.write_char(' ')?;
                e.to_css(dest)?;
                dest.write_char(' ')?;
                f.to_css(dest)?;
                dest.write_char(')')
            }
            SVGTransform::Translate(x, y) => {
                dest.write_str("translate(")?;
                x.to_css(dest)?;
                if !y.is_zero() {
                    dest.write_char(' ')?;
                    y.to_css(dest)?;
                }
                dest.write_char(')')
            }
            SVGTransform::Scale(x, y) => {
                dest.write_str("scale(")?;
                x.to_css(dest)?;
                if x != y {
                    dest.write_char(' ')?;
                    y.to_css(dest)?;
                }
                dest.write_char(')')
            }
            SVGTransform::Rotate(angle, x, y) => {
                dest.write_str("rotate(")?;
                angle.to_css(dest)?;
                if !x.is_zero() || !y.is_zero() {
                    dest.write_char(' ')?;
                    x.to_css(dest)?;
                    dest.write_char(' ')?;
                    y.to_css(dest)?;
                }
                dest.write_char(')')
            }
            SVGTransform::SkewX(angle) => {
                dest.write_str("skewX(")?;
                angle.to_css(dest)?;
                dest.write_char(')')
            }
            SVGTransform::SkewY(angle) => {
                dest.write_str("skewY(")?;
                angle.to_css(dest)?;
                dest.write_char(')')
            }
            SVGTransform::CssTransform(transform) => transform.to_css(dest),
        }
    }
}

impl From<SVGTransform> for Transform {
    fn from(val: SVGTransform) -> Self {
        match val {
            SVGTransform::Matrix(m) => Transform::Matrix(m),
            SVGTransform::Translate(x, y) => {
                Transform::Translate(LengthPercentage::px(x), LengthPercentage::px(y))
            }
            SVGTransform::Scale(x, y) => {
                Transform::Scale(NumberOrPercentage::Number(x), NumberOrPercentage::Number(y))
            }
            SVGTransform::Rotate(angle, x, y) => {
                if x.is_zero() && y.is_zero() {
                    return Transform::Rotate(Angle::Deg(angle));
                }
                let rad = angle.to_radians();
                let cos = rad.cos();
                let sin = rad.sin();
                Transform::Matrix(Matrix {
                    a: cos,
                    b: sin,
                    c: -sin,
                    d: cos,
                    e: (1.0 - cos) * x + sin * y,
                    f: (1.0 - cos) * y - sin * x,
                })
            }
            SVGTransform::SkewX(angle) => Transform::SkewX(Angle::Deg(angle)),
            SVGTransform::SkewY(angle) => Transform::SkewY(Angle::Deg(angle)),
            SVGTransform::CssTransform(transform) => transform,
        }
    }
}

impl TryFrom<&Transform> for SVGTransform {
    type Error = ();

    fn try_from(value: &Transform) -> Result<Self, Self::Error> {
        Ok(match value {
            Transform::Matrix(matrix) => Self::Matrix(matrix.clone()),
            Transform::Translate(
                DimensionPercentage::Dimension(x),
                DimensionPercentage::Dimension(y),
            ) => {
                let Some(x) = x.to_px() else { return Err(()) };
                let Some(y) = y.to_px() else { return Err(()) };
                Self::Translate(x, y)
            }
            Transform::TranslateX(DimensionPercentage::Dimension(x)) => {
                let Some(x) = x.to_px() else { return Err(()) };
                Self::Translate(x, 0.0)
            }
            Transform::Scale(x, y) => {
                let NumberOrPercentage::Number(x) = x else {
                    return Err(());
                };
                let NumberOrPercentage::Number(y) = y else {
                    return Err(());
                };
                Self::Scale(*x, *y)
            }
            Transform::ScaleX(x) => {
                let NumberOrPercentage::Number(x) = x else {
                    return Err(());
                };
                Self::Scale(*x, 0.0)
            }
            Transform::Rotate(angle) => Self::Rotate(angle.to_degrees(), 0.0, 0.0),
            Transform::SkewX(x) => Self::SkewX(x.to_degrees()),
            Transform::SkewY(y) => Self::SkewY(y.to_degrees()),
            t => match t.to_matrix() {
                Some(m) => match m.to_matrix2d() {
                    Some(m) => Self::Matrix(m),
                    None => return Err(()),
                },
                None => return Err(()),
            },
        })
    }
}

impl<'i> Parse<'i> for SVGTransformList {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let mut results = vec![];
        loop {
            input.skip_whitespace();
            input.try_parse(Parser::expect_comma).ok();
            input.skip_whitespace();
            if let Ok(item) = input.try_parse(SVGTransform::parse) {
                results.push(item);
            } else {
                return Ok(SVGTransformList(results));
            }
        }
    }
}

impl ToCss for SVGTransformList {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        for item in &self.0 {
            item.to_css(dest)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// The vector effect to use when drawing an object
///
/// [MDN | vector-effect](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/vector-effect)
pub enum VectorEffect {
    /// This value specifies that no vector effect shall be applied
    None,
    /// The resulting visual effect of this value is that the stroke width is not dependent on the transformations of the element (including non-uniform scaling and shear transformations) and zoom level.
    NonScalingStroke,
    /// The scale of that user coordinate system does not change in spite of any transformation changes from a host coordinate space.
    NonScalingSize,
    /// The rotation and skew of that user coordinate system is suppressed in spite of any transformation changes from a host coordinate space.
    NonRotation,
    /// The position of user coordinate system is fixed in spite of any transformation changes from a host coordinate space.
    FixedPosition,
}

impl<'i> Parse<'i> for VectorEffect {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "none" => Ok(VectorEffect::None),
            "non-scaling-stroke" => Ok(VectorEffect::NonScalingStroke),
            "non-scaling-size" => Ok(VectorEffect::NonScalingSize),
            "non-rotation" => Ok(VectorEffect::NonRotation),
            "fixed-position" => Ok(VectorEffect::FixedPosition),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for VectorEffect {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let ident = match self {
            VectorEffect::None => "none",
            VectorEffect::NonScalingStroke => "non-scaling-stroke",
            VectorEffect::NonScalingSize => "non-scaling-size",
            VectorEffect::NonRotation => "non-rotation",
            VectorEffect::FixedPosition => "fixed-position",
        };
        dest.write_str(ident)
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Specifies the mode for a text element in SVG
///
/// [MDN | writing-mode](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/writing-mode)
pub enum WritingMode {
    /// A top-to-bottom block flow direction.
    HorizontalTb,
    /// A right-to-left block flow direction.
    VerticalRl,
    /// A left-to-right block flow direction.
    VerticalLr,
}

impl<'i> Parse<'i> for WritingMode {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        match_ignore_ascii_case! { ident,
            "horizontal-tb" => Ok(WritingMode::HorizontalTb),
            "vertical-rl" => Ok(WritingMode::VerticalRl),
            "vertical-lr" => Ok(WritingMode::VerticalLr),
            _ => Err(location.new_unexpected_token_error(Token::Ident(ident.clone())))
        }
    }
}

impl ToCss for WritingMode {
    fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        let ident = match self {
            WritingMode::HorizontalTb => "horizontal-tb",
            WritingMode::VerticalRl => "vertical-rl",
            WritingMode::VerticalLr => "vertical-lr",
        };
        dest.write_str(ident)
    }
}

#[derive(Default, Debug)]
/// A mode in which a style can be applied to an element
pub enum Mode {
    #[default]
    /// The application of a style based on an attribute, style attribute, or static stylesheet selector
    Static,
    /// The application of a style based on an at-rule or psuedo-class
    Dynamic,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
/// A style id for a style applied to an element
pub enum Id<'i> {
    /// A CSS property id
    CSS(PropertyId<'i>),
    /// A presentation attribute id
    Attr(PresentationAttrId<'i>),
}

#[derive(Debug, Clone)]
/// A style from either an attribute, style attribute, or static stylesheet selector
pub enum Static<'i> {
    /// A style from a style attribute or static stylesheet selector
    Css(Property<'i>),
    /// A style from an attribute
    Attr(PresentationAttr<'i>),
}

#[derive(Debug, Clone)]
/// A style that can be applied to an element, through either an attribute, style attribute, or stylesheet
pub enum Style<'i> {
    /// The style is declared directly through an attribute, style attribute, or static stylesheet selector
    Static(Static<'i>),
    /// The style is declared within a pseudo-class or at-rule
    Dynamic(Property<'i>),
}

#[derive(Default, Debug)]
/// A collection of the different styles and how they're applied to a given element
pub struct ComputedStyles<'i> {
    /// Inherited styles (e.g. `p`'s color from `<div style="color: red;"><p /></div>`)
    pub inherited: HashMap<Id<'i>, Style<'i>>,
    /// Styles (e.g. `<style>p { color: red }</style>`)
    pub declarations: HashMap<PropertyId<'i>, (u32, Style<'i>)>,
    /// Presentation attributes (e.g. `<p color="red" />`)
    pub attr: HashMap<PresentationAttrId<'i>, Style<'i>>,
    /// Inline styles (e.g. `<p style="color: red;" />`)
    pub inline: HashMap<PropertyId<'i>, Style<'i>>,
    /// Important styles (e.g. `<style>p { color: red !important; }</style>`)
    pub important_declarations: HashMap<PropertyId<'i>, (u32, Style<'i>)>,
    /// Important inline styles (e.g. `<p style="color: red !important;" />`)
    pub inline_important: HashMap<PropertyId<'i>, Style<'i>>,
}

#[cfg(feature = "selectors")]
/// Gathers stylesheet declarations from the document
///
/// # Panics
/// If the internal selector is invalid
pub fn root<'arena, E: Element<'arena>>(root: &E) -> String {
    let output = root
        .select("style")
        .expect("`style` should be a valid selector");
    output
        .filter_map(|e| e.text_content())
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug)]
/// Contains a collection of style data associated with an element
pub struct ElementData<'arena, E: Element<'arena>> {
    inline_style: Option<E::Atom>,
    presentation_attrs: Vec<(
        <<E as node::Node<'arena>>::Name as Name>::LocalName,
        E::Atom,
    )>,
}

impl<'arena, E: Element<'arena>> Default for ElementData<'arena, E> {
    fn default() -> Self {
        Self {
            inline_style: None,
            presentation_attrs: vec![],
        }
    }
}

impl<'arena, E: Element<'arena>> ElementData<'arena, E> {
    /// Create's a map of associated data for every descendant of the given element
    pub fn new(root: &E) -> HashMap<E, Self> {
        let mut styles = HashMap::new();
        for element in root.breadth_first() {
            let inline_style = element
                .get_attribute_local(&"style".into())
                .map(|attr| attr.clone());
            let presentation_attrs = element
                .attributes()
                .into_iter()
                .filter(|a| a.prefix().is_none())
                .map(|a| (a.local_name().clone(), a.value().clone()))
                .collect();
            styles.insert(
                element,
                ElementData {
                    inline_style,
                    presentation_attrs,
                },
            );
        }

        styles
    }
}

impl<'i> ComputedStyles<'i> {
    /// Include all sources of styles
    pub fn with_all<'arena, E: Element<'arena>>(
        self,
        element: &E,
        styles: &Option<StyleSheet<'i, '_>>,
        element_styles: &'i HashMap<E, ElementData<'arena, E>>,
    ) -> ComputedStyles<'i> {
        self.with_inline_style(element, element_styles)
            .with_attribute(element, element_styles)
            .with_style(element, styles)
            .with_inherited(element, styles, element_styles)
    }

    /// Include the computed styles of a parent element
    pub fn with_inherited<'arena, E: Element<'arena>>(
        mut self,
        element: &E,
        styles: &Option<StyleSheet<'i, '_>>,
        element_styles: &'i HashMap<E, ElementData<'arena, E>>,
    ) -> ComputedStyles<'i> {
        let Some(parent) = Element::parent_element(element) else {
            return self;
        };
        let parent_styles = ComputedStyles::default().with_all(&parent, styles, element_styles);
        self.inherited.extend(parent_styles.inherited);
        self.inherited.extend(
            parent_styles
                .attr
                .into_iter()
                .map(|(id, value)| (Id::Attr(id), value)),
        );
        self.inherited.extend(
            parent_styles
                .declarations
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value.1)),
        );
        self.inherited.extend(
            parent_styles
                .inline
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value)),
        );
        self.inherited.extend(
            parent_styles
                .important_declarations
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value.1)),
        );
        self.inherited.extend(
            parent_styles
                .inline_important
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value)),
        );
        self
    }

    /// Include styles from the `style` attribute
    pub fn with_style<'arena, E: Element<'arena>>(
        mut self,
        element: &E,
        styles: &Option<StyleSheet<'i, '_>>,
    ) -> ComputedStyles<'i> {
        let Some(styles) = styles.as_ref() else {
            return self;
        };
        styles
            .rules
            .0
            .iter()
            .for_each(|s| self.with_nested_style(element, s, "", 0, &Mode::Static));
        self
    }

    /// Include a style within a style scope
    fn with_nested_style<'arena, E: Element<'arena>>(
        &mut self,
        element: &E,
        style: &rules::CssRule<'i>,
        selector: &str,
        specificity: u32,
        mode: &Mode,
    ) {
        match style {
            rules::CssRule::Style(r) => r.selectors.0.iter().for_each(|s| {
                let Ok(this_selector) = s.to_css_string(printer::PrinterOptions::default()) else {
                    return;
                };
                let selector = format!("{selector}{this_selector}");
                let Ok(select) = Selector::new::<E>(selector.as_str()) else {
                    return;
                };
                if !select.matches_naive(&SelectElement::new(element.clone())) {
                    return;
                }
                let specificity = specificity + s.specificity();
                self.add_declarations(&r.declarations, specificity, mode);
            }),
            rules::CssRule::Container(rules::container::ContainerRule { rules, .. })
            | rules::CssRule::Media(rules::media::MediaRule { rules, .. }) => {
                rules.0.iter().for_each(|r| {
                    self.with_nested_style(element, r, selector, specificity, &Mode::Dynamic);
                });
            }
            _ => {}
        }
    }

    /// Include styles from a presentable attribute
    fn with_attribute<'arena, E: Element<'arena>>(
        self,
        element: &E,
        element_styles: &'i HashMap<E, ElementData<'arena, E>>,
    ) -> ComputedStyles<'i> {
        let Some(element_styles) = element_styles.get(element) else {
            return self;
        };
        let attr = element_styles
            .presentation_attrs
            .iter()
            .filter_map(|(name, value)| {
                let id = PresentationAttrId::from(name.as_ref());
                let value = PresentationAttr::parse_string(
                    id.clone(),
                    value.as_ref(),
                    ParserOptions::default(),
                )
                .ok()?;
                Some((id, Mode::Static.style(Static::Attr(value))))
            })
            .collect();
        ComputedStyles { attr, ..self }
    }

    fn with_inline_style<'arena, E: Element<'arena>>(
        self,
        element: &E,
        element_styles: &'i HashMap<E, ElementData<'arena, E>>,
    ) -> ComputedStyles<'i> {
        let Some(element_styles) = element_styles.get(element) else {
            return self;
        };
        let Some(inline_styles) = element_styles.inline_style.as_ref() else {
            return self;
        };
        if let Ok(style) = stylesheet::StyleAttribute::parse(
            inline_styles.as_ref(),
            stylesheet::ParserOptions::default(),
        ) {
            let mut inline = HashMap::new();
            let mut inline_important = HashMap::new();
            style.declarations.declarations.iter().for_each(|s| {
                inline.insert(s.property_id(), Mode::Static.style(Static::Css(s.clone())));
            });
            style
                .declarations
                .important_declarations
                .iter()
                .for_each(|s| {
                    inline_important
                        .insert(s.property_id(), Mode::Static.style(Static::Css(s.clone())));
                });

            ComputedStyles {
                inline,
                inline_important,
                ..self
            }
        } else {
            self
        }
    }

    /// Get's a style by id, agnostic of whether it's a presentation attr or css id
    pub fn get(&'i self, id: &Id<'i>) -> Option<&'i Style<'i>> {
        self.get_important(id).or_else(|| self.get_unimportant(id))
    }

    /// Returns whether the given id is resolved by the computed styles.
    pub fn has(&'i self, id: &Id<'i>) -> bool {
        self.get(id).is_some()
    }

    /// Gets the resolved style from a presentation attribute id.
    pub fn get_with_attr(&'i self, id: PresentationAttrId<'i>) -> Option<&'i Style<'i>> {
        let id = Id::Attr(id);
        if let Some(value) = self.get_important(&id) {
            Some(value)
        } else if let Some(value) = self.get_unimportant(&id) {
            Some(value)
        } else {
            None
        }
    }

    /// Gets the resolved style from an id.
    pub fn get_string(&'i self, id: &Id<'i>) -> Option<(Mode, String)> {
        let mut important = false;
        let value = if let Some(value) = self.get_important(id) {
            important = true;
            value
        } else if let Some(value) = self.get_unimportant(id) {
            value
        } else {
            return None;
        };
        let string = value.to_css_string(important)?;
        Some((value.mode(), string))
    }

    fn get_important(&'i self, id: &Id<'i>) -> Option<&'i Style<'i>> {
        match id {
            Id::CSS(id) => {
                if let Some(value) = self.inline_important.get(id) {
                    Some(value)
                } else if let Some((_, value)) = self.important_declarations.get(id) {
                    Some(value)
                } else {
                    None
                }
            }
            Id::Attr(id) => self.get_important(&Id::CSS(id.try_into().ok()?)),
        }
    }

    fn get_unimportant(&'i self, id: &Id<'i>) -> Option<&'i Style<'i>> {
        match id {
            Id::CSS(id) => {
                if let Some(value) = self.inline.get(id) {
                    return Some(value);
                }
                if let Ok(id) = PresentationAttrId::try_from(id) {
                    if let Some(value) = self.attr.get(&id) {
                        return Some(value);
                    }
                }
                if let Some((_, value)) = self.declarations.get(id) {
                    return Some(value);
                }
            }
            Id::Attr(id) => {
                if let Some(value) = self.attr.get(id) {
                    return Some(value);
                }
            }
        }
        self.inherited.get(id)
    }

    /// Gets the resolved static style from an id.
    pub fn get_static<'a>(&'i self, id: &'a Id<'a>) -> Option<&'i Static<'i>>
    where
        'a: 'i,
    {
        match self.get(id) {
            Some(Style::Static(value)) => Some(value),
            _ => None,
        }
    }

    /// Gets the collection of all the resolved styles applied to the element.
    pub fn computed(&'i self) -> HashMap<Id<'i>, &'i Style<'i>> {
        let mut result = HashMap::new();
        let map = |s: &'i (u32, Style<'i>)| &s.1;
        let mut insert = |s: &'i Style<'i>| {
            result.insert(s.id(), s);
        };
        self.attr.values().for_each(&mut insert);
        self.declarations.values().map(map).for_each(&mut insert);
        self.inline.values().for_each(&mut insert);
        self.important_declarations
            .values()
            .map(map)
            .for_each(&mut insert);
        self.inline_important.values().for_each(insert);
        result
    }

    /// Consumed the [`ComputedStyles`] and creates a collection of all the resolved
    /// styles applied to the element.
    pub fn into_computed(self) -> HashMap<Id<'i>, Style<'i>> {
        let mut result = HashMap::new();
        let map = |s: (u32, Style<'i>)| s.1;
        let mut insert = |s: Style<'i>| {
            result.insert(s.id(), s);
        };
        self.inherited.into_values().for_each(&mut insert);
        self.attr.into_values().for_each(&mut insert);
        self.declarations
            .into_values()
            .map(map)
            .for_each(&mut insert);
        self.inline.into_values().for_each(&mut insert);
        self.important_declarations
            .into_values()
            .map(map)
            .for_each(&mut insert);
        self.inline_important.into_values().for_each(insert);
        result
    }

    fn add_declarations(
        &mut self,
        declarations: &declaration::DeclarationBlock<'i>,
        specificity: u32,
        mode: &Mode,
    ) {
        Self::set_declarations(
            &mut self.important_declarations,
            &declarations.important_declarations,
            specificity,
            mode,
        );
        Self::set_declarations(
            &mut self.declarations,
            &declarations.declarations,
            specificity,
            mode,
        );
    }

    fn set_declarations(
        record: &mut HashMap<PropertyId<'i>, (u32, Style<'i>)>,
        declarations: &[properties::Property<'i>],
        specificity: u32,
        mode: &Mode,
    ) {
        for d in declarations {
            let id = d.property_id();
            record.insert(id, (specificity, mode.style(Static::Css(d.clone()))));
        }
    }
}

#[macro_export]
/// Creates a macro called `get_computed_styles` that can be used to get the effective
/// style from [`ComputedStyles`] based on a lightningcss property or presentation attribute
macro_rules! get_computed_styles_factory {
    ($item:ident) => {
        macro_rules! get_computed_styles {
            // NOTE: Branches should be identical, apart from $vp
            ($ident:ident) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident)
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident))
                    .or_else(|| $item.inline.get(&PropertyId::$ident))
                    .or_else(|| $item.declarations.get(&PropertyId::$ident).map(|p| &p.1))
                    .or_else(|| $item.attr.get(&PresentationAttrId::$ident))
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident)))
                    .or_else(|| $item.inherited.get(&Id::Attr(PresentationAttrId::$ident)))
            };
            ($ident:ident ( $vp:expr )) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident($vp))
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident($vp)))
                    .or_else(|| $item.inline.get(&PropertyId::$ident($vp)))
                    .or_else(|| {
                        $item
                            .declarations
                            .get(&PropertyId::$ident($vp))
                            .map(|p| &p.1)
                    })
                    .or_else(|| $item.attr.get(&PresentationAttrId::$ident))
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident($vp))))
                    .or_else(|| $item.inherited.get(&Id::Attr(PresentationAttrId::$ident)))
            };
        }
    };
}

#[macro_export]
/// Creates a macro called `get_computed_property` that can be used to get the effective
/// style from [`ComputedStyles`] based on a lightningcss property
macro_rules! get_computed_property_factory {
    ($item:ident) => {
        macro_rules! get_computed_property {
            // NOTE: Two branches should be identical, apart from $vp
            ($ident:ident) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident)
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident))
                    .or_else(|| $item.inline.get(&PropertyId::$ident))
                    .or_else(|| $item.declarations.get(&PropertyId::$ident).map(|p| &p.1))
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident)))
            };
            ($ident:ident ( $vp:expr )) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident($vp))
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident($vp)))
                    .or_else(|| $item.inline.get(&PropertyId::$ident($vp)))
                    .or_else(|| {
                        $item
                            .declarations
                            .get(&PropertyId::$ident($vp))
                            .map(|p| &p.1)
                    })
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident($vp))))
            };
        }
    };
}

impl<'i> From<&'i str> for Id<'i> {
    fn from(value: &'i str) -> Self {
        let id = PresentationAttrId::from(value);
        if matches!(id, PresentationAttrId::Unknown(_)) {
            Self::CSS(PropertyId::from(value))
        } else {
            Self::Attr(id)
        }
    }
}

impl<'i> Static<'i> {
    /// Returns the id of a style
    pub fn id(&self) -> Id<'i> {
        match self {
            Self::Css(property) => Id::CSS(property.property_id()),
            Self::Attr(attr) => Id::Attr(attr.presentation_attr_id()),
        }
    }

    /// Serializes the value of a style as a string
    pub fn to_css_string(&self, important: bool, options: PrinterOptions) -> Option<String> {
        match self {
            Self::Css(property) => property.value_to_css_string(options).ok().map(|mut s| {
                if important {
                    s.write_str("!important").ok();
                }
                s
            }),
            Self::Attr(attr) => attr.value_to_css_string(options).ok(),
        }
    }
}

impl<'i> Style<'i> {
    /// Returns the style as a static style
    pub fn inner(&self) -> Static<'i> {
        match self {
            Self::Static(v) => v.clone(),
            Self::Dynamic(v) => Static::Css(v.clone()),
        }
    }

    /// Returns the id of a style
    pub fn id(&self) -> Id<'i> {
        self.inner().id()
    }

    /// Returns the mode in which the styles are applied to an element. i.e. static or dynamic
    pub fn mode(&self) -> Mode {
        match self {
            Self::Static(_) => Mode::Static,
            Self::Dynamic(_) => Mode::Dynamic,
        }
    }

    /// Returns whether the style was part of an attribute or non-dynamic selector
    pub fn is_static(&self) -> bool {
        self.mode().is_static()
    }

    /// Returns whether the style was part of an at-rule or pseudo-class
    pub fn is_dynamic(&self) -> bool {
        self.mode().is_dynamic()
    }

    /// Returns whether the style wasn't able to be parsed
    pub fn is_unparsed(&self) -> bool {
        match self {
            Self::Static(style) => match style {
                Static::Css(css) => matches!(css, Property::Unparsed(_)),
                Static::Attr(attr) => matches!(attr, PresentationAttr::Unparsed(_)),
            },
            Self::Dynamic(css) => matches!(css, Property::Unparsed(_)),
        }
    }

    /// Serializes the value of a style as a string
    pub fn to_css_string(&self, important: bool) -> Option<String> {
        self.inner()
            .to_css_string(important, PrinterOptions::default())
    }

    /// Gets the presentation attribute representation if the style is sourced from an attribute
    pub fn presentation_attr(&self) -> Option<PresentationAttr<'i>> {
        match self.inner() {
            Static::Attr(attr) => Some(attr),
            Static::Css(_) => None,
        }
    }

    /// Gets the lightningcss representation if the style is sourced from a stylesheet
    pub fn property(&self) -> Option<Property<'i>> {
        match self.inner() {
            Static::Css(css) => Some(css),
            Static::Attr(_) => None,
        }
    }
}

impl Mode {
    /// # Panics
    /// If attempting to assign attribute to dynamic style
    pub fn style<'i>(&self, style: Static<'i>) -> Style<'i> {
        match self {
            Self::Static => Style::Static(style),
            Self::Dynamic => match style {
                Static::Attr(_) => panic!("cannot style attr as dynamic"),
                Static::Css(property) => Style::Dynamic(property),
            },
        }
    }

    /// Returns whether the source of a style is from an attribute or not
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static)
    }

    /// Returns whether the source of a style is from a stylesheet or not
    pub fn is_dynamic(&self) -> bool {
        !self.is_static()
    }
}
