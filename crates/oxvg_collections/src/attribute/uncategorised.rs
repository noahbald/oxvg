//! Miscellaneous attribute types
use lightningcss::media_query::MediaList;

#[cfg(feature = "parse")]
use oxvg_parse::{error::Error, Parse, Parser};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};

use crate::{atom::Atom, enum_attr};

use super::{
    core::{Angle, Anything, Number, Percentage},
    presentation::{LengthOrNumber, LengthPercentage},
    transform::SVGTransform,
};

enum_attr!(
    /// Describes how overlapping colours should blend
    ///
    /// [MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/blend-mode)
    /// [w3](https://www.w3.org/TR/compositing-1/#ltblendmodegt)
    #[derive(Default)]
    BlendMode {
        /// Blended like overlapping pieces of paper
        #[default]
        Normal: "normal",
        /// Blended like images printed on transparent film overlapping.
        Multiply: "multiply",
        /// Blended like `multiply`, but the foreground only needs to be as dark as the inverse of the backdrop to make the final image black.
        ColorBurn: "color-burn",
        /// Blended like two images shining onto a projection screen.
        Screen: "screen",
        /// Blended like `screen`, but the foreground only needs to be as light as the inverse of the backdrop to create a fully lit color.
        ColorDodge: "color-dodge",
        /// Blended like shining a hard spotlight on the backdrop.
        HardLight: "hard-light",
        /// Blended like `hard-light`, but with the layers swapped.
        Overlay: "overlay",
        /// Blended like shining a diffused spotlight on the backdrop.
        SoftLight: "soft-light",
        /// The final color is composed of the darkest values of each color channel.
        Darken: "darken",
        /// The final color is composed of the lightest values of each color channel.
        Lighten: "lighten",
        /// The final color is the result of subtracting the darker of the two colors from the lighter one.
        Difference: "difference",
        /// Blended like `difference`, but with less contrast.
        Exclusion: "exclusion",
        /// The final color has the hue of the top color, while using the saturation and luminosity of the bottom color.
        Hue: "hue",
        /// The final color has the saturation of the top color, while using the hue and luminosity of the bottom color.
        Saturation: "saturation",
        /// The final color has the hue and saturation of the top color, while using the luminosity of the bottom color.
        Color: "color",
        /// The final color has the luminosity of the top color, while using the hue and saturation of the bottom color.
        Luminosity: "luminosity",
    }
);
enum_attr!(
    /// Sets the CORS credentials configuration for fetched data
    ///
    /// All the following items are considered to be credentials:
    /// - HTTP cookies
    /// - TLS client certificates
    /// - The Authorization and Proxy-Authorization headers.
    ///
    /// [MDN | Credentials information](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch#including_credentials)
    /// [w3](https://www.w3.org/TR/filter-effects-1/#element-attrdef-feimage-crossorigin)
    CrossOrigin {
        /// CORS credentials is set to `"same-origin"`
        Anonymous: "anonymous",
        /// CORS credentials is set to `"include"`
        UseCredentials: "use-credentials",
    }
);

#[derive(Clone, Debug, PartialEq, Eq)]
/// Controls how graphics stretch to fill an SVG's viewport
///
/// [w3](https://svgwg.org/svg2-draft/coords.html#PreserveAspectRatioAttribute)
#[derive(Default)]
pub struct PreserveAspectRatio {
    /// Alignment while scaling
    pub align: PreserveAspectRatioAlign,
    /// Condition to complete scaling
    pub meet_or_slice: PreserveAspectRatioMeetOrSlice,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for PreserveAspectRatio {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let align = PreserveAspectRatioAlign::parse(input)?;
        input.skip_whitespace();
        let mut result = Self {
            align,
            meet_or_slice: input
                .try_parse(PreserveAspectRatioMeetOrSlice::parse)
                .unwrap_or_default(),
        };
        if result.align == PreserveAspectRatioAlign::None {
            result.meet_or_slice = PreserveAspectRatioMeetOrSlice::Meet;
        }
        Ok(result)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for PreserveAspectRatio {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.align.write_value(dest)?;
        if self.meet_or_slice != PreserveAspectRatioMeetOrSlice::default() {
            dest.write_char(' ')?;
            self.meet_or_slice.write_value(dest)?;
        }
        Ok(())
    }
}
#[test]
fn preserve_aspect_ratio() {
    use oxvg_parse::Parse as _;
    assert_eq!(
        PreserveAspectRatio::parse_string("none"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::None,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("none slice"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::None,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMinYMin"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMinYMin,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMidYMin meet"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMidYMin,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMidYMin slice"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMidYMin,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Slice
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMaxYMin"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMaxYMin,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMinYMid meet"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMinYMid,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMidYMid slice"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMidYMid,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Slice
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMaxYMid"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMaxYMid,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMinYMax meet"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMinYMax,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMidYMax slice"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMidYMax,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Slice
        })
    );
    assert_eq!(
        PreserveAspectRatio::parse_string("xMaxYMax"),
        Ok(PreserveAspectRatio {
            align: PreserveAspectRatioAlign::XMaxYMax,
            meet_or_slice: PreserveAspectRatioMeetOrSlice::Meet
        })
    );

    assert!(PreserveAspectRatio::parse_string("meet").is_err());
    assert_eq!(
        PreserveAspectRatio::parse_string("xMinYMin unknown"),
        Err(Error::ExpectedDone)
    );
}

enum_attr!(
    /// Controls the method of alignment while scaling and preserving aspect-ratio
    #[derive(Default)]
    PreserveAspectRatioAlign {
        /// Scale edges non-uniformly until edges meet the bounds of a viewbox
        None: "none",
        /// Scale edges uniformly from the minimum x/y value of the viewport
        XMinYMin: "xMinYMin",
        /// Scale edges uniformly from the middle x and minimum y value of the viewport
        XMidYMin: "xMidYMin",
        /// Scale edges uniformly from the maximum x and minimum y value of the viewport
        XMaxYMin: "xMaxYMin",
        /// Scale edges uniformly from the minimum x and middle y value of the viewport
        XMinYMid: "xMinYMid",
        /// Scale edges uniformly from the middle x/y value of the viewport
        #[default]
        XMidYMid: "xMidYMid",
        /// Scale edges uniformly from the maximum x and middle y value of the viewport
        XMaxYMid: "xMaxYMid",
        /// Scale edges uniformly from the minimum x and maximum y value of the viewport
        XMinYMax: "xMinYMax",
        /// Scale edges uniformly from the middle x and maximum y value of the viewport
        XMidYMax: "xMidYMax",
        /// Scale edges uniformly from the maximum x and maximum y value of the viewport
        XMaxYMax: "xMaxYMax",
    }
);

enum_attr!(
    /// Controls the scaling of graphics when resizing to preserve aspect-ratio
    #[derive(Default)]
    PreserveAspectRatioMeetOrSlice {
        /// Scales the image up until an edge meets the bounds of a viewbox
        ///
        /// ```txt
        /// +--------+
        /// |  +--+  |
        /// |  +--+  |
        /// +--------|
        /// ```
        #[default]
        Meet: "meet",
        /// Scale the image down until an edge meets the bounds of a viewbox
        ///
        /// ```txt
        ///   +----+
        /// +--------+
        /// | |    | |
        /// | |    | |
        /// +--------|
        ///   +----+
        /// ```
        Slice: "slice",
    }
);

enum_attr!(
    /// Indicates whether it is possible to seek backwards in the document.
    #[derive(Default)]
    Playbackorder {
        /// This file is intended to be played only in the forward direction.
        Forwardonly: "forwardonly",
        #[default]
        /// The document is authored for seeking in both directions.
        All: "all",
    }
);

enum_attr!(
    /// Controls the initialization of the timeline.
    Timelinebegin {
        /// The timeline starts when the load event triggers.
        Loadend: "loadend",
        /// The timeline starts when the SVG is fully parsed.
        Loadbegin: "loadbegin",
    }
);

enum_attr!(
    /// Defines the coordinate system for an element.
    ///
    /// [w3](https://drafts.fxtf.org/css-masking/#element-attrdef-clippath-clippathunits)
    #[derive(Default)]
    Units {
        /// Values are absolute values relative to a reference box (i.e. the user coordinate system)
        ///
        /// [w3 | user coordinate system](https://drafts.csswg.org/css-transforms-1/#user-coordinate-system)
        #[default]
        UserSpaceOnUse: "userSpaceOnUse",
        /// Values are relative units relative to the element's bounding box
        ObjectBoundingBox: "objectBoundingBox",
    }
);

enum_attr!(
    /// Defines the coordinate system for an element.
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/painting.html#MarkerUnitsAttribute)
    /// [w3 | SVG 2](https://svgwg.org/svg2-draft/painting.html#MarkerUnitsAttribute)
    #[derive(Default)]
    MarkerUnits {
        /// Values are absolute values relative to a reference box (i.e. the user coordinate system)
        ///
        /// [w3 | user coordinate system](https://drafts.csswg.org/css-transforms-1/#user-coordinate-system)
        #[default]
        UserSpaceOnUse: "userSpaceOnUse",
        /// Values have a single unit equal to the size in user units of the painted stroke width of the element referencing the marker
        StrokeWidth: "strokeWidth",
    }
);

enum_attr!(
    /// How the text is stretched or compressed to fit the width defined by the textLength attribute.
    ///
    /// [w3](https://svgwg.org/svg2-draft/text.html#TextElementLengthAdjustAttribute)
    LengthAdjust {
        /// Only the spacing between glyphs is adjusted.
        Spacing: "spacing",
        /// Both the spacing between glyphs and the glyphs themselves are stretched/compressed.
        SpacingAndGlyphs: "spacingAndGlyphs",
    }
);

enum_attr!(
    /// Defines the relationship between a linked resource and the current document.
    ///
    /// [MDN](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Attributes/rel)
    /// [w3](https://svgwg.org/svg2-draft/linking.html#AElementRelAttribute)
    LinkType {
        /// Alternate representations of the current document.
        Alternate: "alternate",
        /// Author of the current document or article.
        Author: "author",
        /// Permalink for the nearest ancestor section.
        Bookmark: "bookmark",
        /// Preferred URL for the current document.
        Canonical: "canonical",
        /// Link to a compression dictionary that can be used to compress future downloads for resources on this site.
        CompressionDictionary: "compression-dictionary",
        /// Tells the browser to preemptively perform DNS resolution for the target resource's origin.
        DnsPrefetch: "dns-prefetch",
        /// The referenced document is not part of the same site as the current document.
        External: "external",
        /// Allows the page to be render-blocked until the essential parts of the document are parsed so it will render consistently.
        Expect: "expect",
        /// Link to context-sensitive help.
        Help: "help",
        /// An icon representing the current document.
        Icon: "icon",
        /// The current document is covered by the copyright license described by the referenced document.
        License: "license",
        /// Web app manifest.
        Manifest: "manifest",
        /// Indicates that the current document represents the person who owns the linked content.
        Me: "me",
        /// Tells to browser to preemptively fetch the script and store it in the document's module map for later evaluation.
        Modulepreload: "modulepreload",
        /// Indicates that the current document is a part of a series and that the next document in the series is the referenced document.
        Next: "next",
        /// Indicates that the current document's original author or publisher does not endorse the referenced document.
        Nofollow: "nofollow",
        /// Creates a top-level browsing context that is not an auxiliary browsing context.
        Noopener: "noopener",
        /// No Referer header will be included.
        Noreferrer: "noreferrer",
        /// Creates an auxiliary browsing context.
        Opener: "opener",
        /// Gives the address of the pingback server that handles pingbacks to the current document.
        Pingback: "pingback",
        /// Preemptively connect to the target resource's origin.
        Preconnect: "preconnect",
        /// Preemptively fetch and cache the target resource as it is likely to be required for a followup navigation.
        Prefetch: "prefetch",
        /// preemptively fetch and cache the target resource for current navigation.
        Preload: "preload",
        /// reemptively fetch the target resource and process it in a way that helps deliver a faster response in the future.
        Prerender: "prerender",
        /// Indicates that the current document is a part of a series and that the previous document in the series is the referenced document.
        Prev: "prev",
        /// Gives a link to a information about the data collection and usage practices that apply to the current document.
        PrivacyPolicy: "privacy-policy",
        /// Gives a link to a resource that can be used to search through the current document and its related pages.
        Search: "search",
        /// Imports a style sheet.
        Stylesheet: "stylesheet",
        /// Gives a tag (identified by the given address) that applies to the current document.
        Tag: "tag",
        /// Link to the agreement, or terms of service, between the document's provider and users who wish to use the document.
        TermsOfService: "terms-of-service",
    }
);

/// A media/mime type
///
/// [w3](https://svgwg.org/svg2-draft/interact.html#ScriptElementTypeAttribute)
pub type MediaType<'i> = Anything<'i>;

#[derive(Debug, Clone, PartialEq)]
/// A media query
///
/// [w3](https://svgwg.org/svg2-draft/styling.html#StyleElementMediaAttribute)
pub struct MediaQueryList<'i>(pub MediaList<'i>);
#[cfg(feature = "parse")]
impl<'input> oxvg_parse::Parse<'input> for MediaQueryList<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        MediaList::parse(&mut cssparser_lightningcss::Parser::new(
            &mut cssparser_lightningcss::ParserInput::new(input.take_slice()),
        ))
        .map(Self)
        .map_err(Error::Lightningcss)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for MediaQueryList<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_value(dest)
    }
}
#[test]
fn media_query_list() {
    use oxvg_parse::Parse as _;
    MediaQueryList::parse_string("screen and (width >= 900px)").unwrap();
}

#[derive(Clone, Debug, PartialEq)]
/// A number or percentage value
pub enum NumberPercentage {
    /// A number
    Number(Number),
    /// Percentage
    Percentage(Percentage),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for NumberPercentage {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| Percentage::parse(input).map(Self::Percentage))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for NumberPercentage {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_value(dest),
            Self::Percentage(percentage) => percentage.write_value(dest),
        }
    }
}
#[test]
fn number_percentage() {
    use oxvg_parse::Parse as _;
    assert_eq!(
        NumberPercentage::parse_string("10"),
        Ok(NumberPercentage::Number(10.0))
    );
    assert_eq!(
        NumberPercentage::parse_string("10%"),
        Ok(NumberPercentage::Percentage(Percentage(0.1)))
    );

    assert_eq!(
        NumberPercentage::parse_string("10px"),
        Err(Error::ExpectedDone)
    );
}

#[derive(Clone, Debug, PartialEq)]
/// Indicate rotation when an element is placed on a shape.
///
/// [w3](https://svgwg.org/svg2-draft/painting.html#OrientAttribute)
pub enum Orient {
    /// The element is rotated relative to the direction of the path on the shape it's placed.
    Auto,
    /// The element is rotated opposite to the direction of the path on the shape it's placed.
    AutoStartReverse,
    /// The element is rotated exclusively in the direction specified
    Angle(Angle),
    /// The element is rotated exclusively in the direction specified, in degrees
    Number(Number),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Orient {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| {
                input.expect_ident().map_err(|_| ()).and_then(|ident| {
                    let ident: &str = ident;
                    Ok(match ident {
                        "auto" => Self::Auto,
                        "auto-start-reverse" => Self::AutoStartReverse,
                        _ => return Err(()),
                    })
                })
            })
            .or_else(|()| input.try_parse(|input| Angle::parse(input).map(Self::Angle)))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Orient {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::AutoStartReverse => dest.write_str("auto-start-reverse"),
            Self::Angle(angle) => angle.write_value(dest),
            Self::Number(number) => number.write_value(dest),
        }
    }
}
#[test]
fn orient() {
    use oxvg_parse::Parse as _;
    assert_eq!(Orient::parse_string("auto"), Ok(Orient::Auto));
    assert_eq!(
        Orient::parse_string("auto-start-reverse"),
        Ok(Orient::AutoStartReverse)
    );
    assert_eq!(
        Orient::parse_string("90deg"),
        Ok(Orient::Angle(Angle::Deg(90.0)))
    );
    assert_eq!(Orient::parse_string("90"), Ok(Orient::Number(90.0)));

    assert_eq!(Orient::parse_string("90px"), Err(Error::ExpectedDone));
}

enum_attr!(
    /// This property has no effect
    ///
    /// [w3](https://svgwg.org/specs/animations/#OriginAttribute)
    Origin {
        /// The default origin
        Default: "default"
    }
);

#[derive(Clone, Debug, PartialEq)]
/// The radius of a shape
///
/// [w3](https://svgwg.org/svg2-draft/geometry.html#RxProperty)
pub enum Radius {
    /// The length of the radius
    LengthPercentage(LengthPercentage),
    /// The length inherits the length of some other attribute
    Auto,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Radius {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| input.expect_ident_matching("auto").map(|()| Self::Auto))
            .or_else(|_| LengthPercentage::parse(input).map(Self::LengthPercentage))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Radius {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::LengthPercentage(v) => v.write_value(dest),
        }
    }
}
#[test]
fn radius() {
    use oxvg_parse::Parse as _;
    assert_eq!(Radius::parse_string("auto"), Ok(Radius::Auto));
    assert_eq!(
        Radius::parse_string("20px"),
        Ok(Radius::LengthPercentage(LengthPercentage(
            lightningcss::values::percentage::DimensionPercentage::Dimension(
                lightningcss::values::length::LengthValue::Px(20.0)
            )
        )))
    );
}

/// The name which is used as the first parameter for icc-color specifications
///
/// [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/color.html#ColorProfileElementNameAttribute)
#[derive(Clone, Debug, PartialEq, Default)]
pub enum ColorProfileName<'input> {
    #[default]
    /// Case-insensitive name of the pre-defined standard colour profile.
    Srbg,
    /// The name of an icc-color specification.
    Name(Atom<'input>),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for ColorProfileName<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        Ok(input
            .try_parse(|input| {
                let ident = input.expect_ident().map_err(|_| ())?.to_lowercase();
                if ident == "srgb" {
                    Ok(Self::Srbg)
                } else {
                    Err(())
                }
            })
            .unwrap_or_else(|()| Self::Name(input.take_slice().into())))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for ColorProfileName<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Srbg => dest.write_str("sRGB"),
            Self::Name(name) => name.write_value(dest),
        }
    }
}
#[test]
fn color_profile_name() {
    use oxvg_parse::Parse as _;
    assert_eq!(
        ColorProfileName::parse_string("sRGB"),
        Ok(ColorProfileName::Srbg)
    );
    assert_eq!(
        ColorProfileName::parse_string("srgb"),
        Ok(ColorProfileName::Srbg)
    );
    assert_eq!(
        ColorProfileName::parse_string("name"),
        Ok(ColorProfileName::Name("name".into()))
    );
}

enum_attr!(
    /// Permits the specification of a color profile rendering intent other than the default.
    ///
    /// [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/color.html#ColorProfileElementRenderingIntentAttribute)
    #[derive(Default)]
    RenderingIntent {
        /// This is the default behavior.
        #[default]
        Auto: "auto",
        /// Preserves the relationship between colors.
        Perceptual: "perceptual",
        /// Leaves colors that fall inside the gamut unchanged.
        RelativeColorimetric: "relative-colorimetric",
        /// Preserves the relative saturation (chroma) values of the original pixels.
        Saturation: "saturation",
        /// Disables white point matching when converting colors.
        AbsoluteColorimetric: "absolute-colorimetric",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// Defines the reference point of the symbol which is to be placed exactly at the symbol's x,y positioning coordinate.
///
/// [w3](https://svgwg.org/svg2-draft/struct.html#SymbolElementRefYAttribute)
pub enum RefX {
    /// The position of the marker on the shape
    LengthOrNumber(LengthOrNumber),
    /// The left edge of the shape
    Left,
    /// The horizontal center of the shape
    Center,
    /// The right edge of the shape
    Right,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for RefX {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| {
                let ident: &str = input.expect_ident()?;
                Ok(match ident {
                    "left" => Self::Left,
                    "center" => Self::Center,
                    "right" => Self::Right,
                    received => {
                        return Err(Error::ExpectedIdent {
                            expected: "one of `left` `center` `right` or length",
                            received,
                        })
                    }
                })
            })
            .or_else(|e| {
                input.skip_whitespace();
                LengthOrNumber::parse(input)
                    .map(Self::LengthOrNumber)
                    .map_err(|_| e)
            })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for RefX {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::LengthOrNumber(length_or_number) => length_or_number.write_value(dest),
            Self::Left => dest.write_str("left"),
            Self::Center => dest.write_str("center"),
            Self::Right => dest.write_str("right"),
        }
    }
}
#[test]
fn ref_x() {
    use oxvg_parse::Parse as _;
    assert_eq!(RefX::parse_string("left"), Ok(RefX::Left));
    assert_eq!(RefX::parse_string("center"), Ok(RefX::Center));
    assert_eq!(RefX::parse_string("right"), Ok(RefX::Right));
    assert_eq!(
        RefX::parse_string("10"),
        Ok(RefX::LengthOrNumber(LengthOrNumber::Number(10.0)))
    );
    assert_eq!(
        RefX::parse_string("10px"),
        Ok(RefX::LengthOrNumber(LengthOrNumber::Length(
            lightningcss::values::length::Length::Value(
                lightningcss::values::length::LengthValue::Px(10.0)
            )
        )))
    );

    assert_eq!(
        RefX::parse_string("top"),
        Err(Error::ExpectedIdent {
            expected: "one of `left` `center` `right` or length",
            received: "top"
        })
    );
}

#[derive(Clone, Debug, PartialEq)]
/// Defines the reference point of the symbol which is to be placed exactly at the symbol's x,y positioning coordinate.
///
/// [w3](https://svgwg.org/svg2-draft/struct.html#SymbolElementRefYAttribute)
pub enum RefY {
    /// The position of the marker on the shape
    LengthOrNumber(LengthOrNumber),
    /// The top edge of the shape
    Top,
    /// The vertical center of the shape
    Center,
    /// The bottom edge of the shape
    Bottom,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for RefY {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| {
                let ident: &str = input.expect_ident()?;
                Ok(match ident {
                    "top" => Self::Top,
                    "center" => Self::Center,
                    "bottom" => Self::Bottom,
                    received => {
                        return Err(Error::ExpectedIdent {
                            expected: "one of `top` `center` `bottom` or length",
                            received,
                        })
                    }
                })
            })
            .or_else(|e| {
                input.skip_whitespace();
                LengthOrNumber::parse(input)
                    .map(Self::LengthOrNumber)
                    .map_err(|_| e)
            })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for RefY {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::LengthOrNumber(length_or_number) => length_or_number.write_value(dest),
            Self::Top => dest.write_str("top"),
            Self::Center => dest.write_str("center"),
            Self::Bottom => dest.write_str("bottom"),
        }
    }
}
#[test]
fn ref_y() {
    use oxvg_parse::Parse as _;
    assert_eq!(RefY::parse_string("top"), Ok(RefY::Top));
    assert_eq!(RefY::parse_string("center"), Ok(RefY::Center));
    assert_eq!(RefY::parse_string("bottom"), Ok(RefY::Bottom));
    assert_eq!(
        RefY::parse_string("10"),
        Ok(RefY::LengthOrNumber(LengthOrNumber::Number(10.0)))
    );
    assert_eq!(
        RefY::parse_string("10px"),
        Ok(RefY::LengthOrNumber(LengthOrNumber::Length(
            lightningcss::values::length::Length::Value(
                lightningcss::values::length::LengthValue::Px(10.0)
            )
        )))
    );

    assert_eq!(
        RefY::parse_string("left"),
        Err(Error::ExpectedIdent {
            expected: "one of `top` `center` `bottom` or length",
            received: "left"
        })
    );
}

enum_attr!(
    /// A referrer policy string. Controls whether the current URL will be attached to requests.
    ///
    /// A referrer is derived from the current URL, and as such
    /// - The current URL is the resource being viewed, e.g. `https://origin.com/path?query=string&username=john&password=1234#fragment`
    /// - The referrer URL is a stripped version of the current URL, e.g. `https://origin.com/path?query=string&username=john&password=1234#fragment` becomes `https://origin.com/path?query=string`
    /// - The origin, e.g. `origin.com` of `origin.com/path#hash?query=string`
    /// - A URL is potentially-trustworthy when it's protocol/scheme is `https://` or `about:`
    ///
    /// # Security
    ///
    /// Referrers can potential contain sensitive information, so the referrer policy should be given
    /// when the current page does not already have a referrer-policy and it used on URLs that are
    /// unknown or may contain sensitive information.
    ///
    /// [w3](https://w3c.github.io/webappsec-referrer-policy/#referrer-policy)
    ReferrerPolicy {
        /// No referrer information is to be sent along with requests to any origin.
        NoReferrer: "no-referrer",
        /// Sent when:
        ///
        /// - The referrer URL and current URL are both potentially trustworthy URLs, or
        /// - The referrer URL is a non-potentially trustworthy URL.
        NoReferrerWhenDowngrade: "no-referrer-when-downgrade",
        /// Sent when making same-origin-referrer requests.
        SameOrigin: "same-origin",
        /// Sent when making both same-origin-referrer requests and cross-origin-referrer requests.
        Origin: "origin",
        /// Sends the origin when:
        ///
        /// - The referrer URL and current URL are both potentially trustworthy URLs, or
        /// - The referrer URL is a non-potentially trustworthy URL.
        StrictOrigin: "strict-origin",
        /// Sent on same-origin-referrer requests, or only the origin when making cross-origin-referrer requests.
        OriginWhenCrossOrigin: "origin-when-cross-origin",
        /// Sent on same-origin-referrer requests, or only the origin when making cross-origin-referrer requests when:
        ///
        /// - The referrer URL and current URL are both potentially trustworthy URLs, or
        /// - The referrer URL is a non-potentially trustworthy URL.
        StrictOriginWhenCrossOrigin: "strict-origin-when-cross-origin",
        /// Sent for both same-origin-referrer requests and cross-origin-referrer requests.
        UnsafeUrl: "unsafe-url",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// Post-multiplies a supplemental transformation matrix.
///
/// [w3](https://svgwg.org/specs/animations/#RotateAttribute)
pub enum Rotate {
    /// A constant rotation transformation, where the rotation angle is the specified number of degrees.
    Number(Number),
    /// The object is rotated over time by the angle of the direction of the motion path.
    Auto,
    /// The object is rotated over time by the angle of the direction of the motion path plus 180 degrees.
    AutoReverse,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Rotate {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| {
                let ident: &str = input.expect_ident().map_err(|_| ())?;
                Ok(match ident {
                    "auto" => Self::Auto,
                    "auto-reverse" => Self::AutoReverse,
                    _ => return Err(()),
                })
            })
            .or_else(|()| {
                input.skip_whitespace();
                Number::parse(input).map(Self::Number)
            })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Rotate {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::AutoReverse => dest.write_str("auto-reverse"),
            Self::Number(number) => number.write_value(dest),
        }
    }
}
#[test]
fn rotate() {
    use oxvg_parse::Parse as _;
    assert_eq!(Rotate::parse_string("auto"), Ok(Rotate::Auto));
    assert_eq!(
        Rotate::parse_string("auto-reverse"),
        Ok(Rotate::AutoReverse)
    );
    assert_eq!(Rotate::parse_string("10"), Ok(Rotate::Number(10.0)));
}

enum_attr!(
    /// Indicates what happens if the gradient starts or ends inside the bounds of the target rectangle.
    ///
    /// [w3](https://svgwg.org/svg2-draft/pservers.html#LinearGradientElementSpreadMethodAttribute)
    SpreadMethod {
        /// Use the terminal colors of the gradient to fill the remainder of the target region.
        Pad: "pad",
        /// Reflect the gradient pattern continuously until the target rectangle is filled.
        Reflect: "reflect",
        /// Repeat the gradient pattern continuously until the target region is filled.
        Repeat: "repeat",
    }
);

#[derive(Clone, Debug, PartialEq, Eq, Default)]
/// specifies the name of the browsing context into which a document is to be opened when the link is activated
///
/// [w3](https://svgwg.org/svg2-draft/linking.html#AElementTargetAttribute)
pub enum Target<'input> {
    #[default]
    /// The current SVG image is replaced by the linked content
    _Self,
    /// The immediate parent browsing context of the SVG image is replaced
    _Parent,
    /// The content of the full active window or tab is replaced
    _Top,
    /// A new un-named window or tab is requested for the display
    _Blank,
    /// Specifies the name of the browsing context (tab, inline frame, object, etc.) for display of the linked content. If a context with this name already exists, and can be securely accessed from this document, it is re-used, replacing the existing content.
    XMLName(Atom<'input>),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Target<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        Ok(input
            .try_parse(|input| {
                let ident: &str = input.expect_ident().map_err(|_| ())?;
                Ok(match ident {
                    "_self" => Self::_Self,
                    "_parent" => Self::_Parent,
                    "_top" => Self::_Top,
                    "_blank" => Self::_Blank,
                    _ => return Err(()),
                })
            })
            .unwrap_or_else(|()| Self::XMLName(input.take_slice().into())))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Target<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::_Self => dest.write_str("_self"),
            Self::_Parent => dest.write_str("_parent"),
            Self::_Top => dest.write_str("_top"),
            Self::_Blank => dest.write_str("_blank"),
            Self::XMLName(name) => name.write_value(dest),
        }
    }
}
#[test]
fn target() {
    use oxvg_parse::Parse as _;
    assert_eq!(Target::parse_string("_self"), Ok(Target::_Self));
    assert_eq!(Target::parse_string("_parent"), Ok(Target::_Parent));
    assert_eq!(Target::parse_string("_top"), Ok(Target::_Top));
    assert_eq!(Target::parse_string("_blank"), Ok(Target::_Blank));

    assert_eq!(
        Target::parse_string("_Self"),
        Ok(Target::XMLName("_Self".into()))
    );
    assert_eq!(
        Target::parse_string("name"),
        Ok(Target::XMLName("name".into()))
    );
}

enum_attr!(
    /// Indicates the method by which text should be rendered along the path.
    ///
    /// [w3](https://svgwg.org/svg2-draft/text.html#TextPathElementMethodAttribute)
    TextPathMethod {
        /// Indicates that the typographic character should be rendered using simple 2×3 matrix transformations
        Align: "align",
        /// Indicates that the typographic character outlines will be converted into paths, and then all end points and control points will be adjusted
        Stretch: "stretch",
    }
);

enum_attr!(
    /// Indicates how the user agent should determine the spacing between typographic characters.
    ///
    /// [w3](https://svgwg.org/svg2-draft/text.html#TextPathElementSpacingAttribute)
    TextPathSpacing {
        /// Indicates that the user agent should use text-on-a-path layout algorithms
        Auto: "auto",
        /// indicates that the typographic characters should be rendered exactly according to the spacing rules as specified in [Text on a path layout rules](https://svgwg.org/svg2-draft/text.html#TextpathLayoutRules).
        Exact: "exact",
    }
);

enum_attr!(
    /// Determines the side of the path the text is placed on.
    ///
    /// [w3](https://svgwg.org/svg2-draft/text.html#TextPathElementSideAttribute)
    TextPathSide {
        /// This value places the text on the left side of the path
        Left: "left",
        /// This value places the text on the right side of the path (relative to the path direction).
        Right: "right",
    }
);

/// A transform definition.
///
/// [w3 (SVG 1.1 uses it's own transform syntax)](https://www.w3.org/TR/SVG11/coords.html#TransformAttribute)
/// [w3 (SVG 2 uses css transform)](https://svgwg.org/svg2-draft/coords.html#TransformProperty)
pub type Transform = SVGTransform;

#[derive(Clone, Debug, PartialEq)]
/// Value representing either true or false.
///
/// [w3](https://www.w3.org/TR/wai-aria-1.1/#valuetype_true-false)
pub struct TrueFalse(pub bool);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for TrueFalse {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let str = input.expect_ident()?;
        Ok(Self(match str {
            "true" => true,
            "false" => false,
            received => {
                return Err(Error::ExpectedIdent {
                    expected: "one of `true` `false`",
                    received,
                })
            }
        }))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for TrueFalse {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        if self.0 {
            dest.write_str("true")
        } else {
            dest.write_str("false")
        }
    }
}
#[test]
fn true_false() {
    use oxvg_parse::Parse as _;
    assert_eq!(TrueFalse::parse_string("true"), Ok(TrueFalse(true)));
    assert_eq!(TrueFalse::parse_string("false"), Ok(TrueFalse(false)));

    assert_eq!(
        TrueFalse::parse_string("other"),
        Err(Error::ExpectedIdent {
            expected: "one of `true` `false`",
            received: "other"
        })
    );
}

#[derive(Clone, Debug, PartialEq)]
/// Value representing true, false, or not applicable.
///
/// [w3](https://www.w3.org/TR/wai-aria-1.1/#valuetype_true-false-undefined)
pub struct TrueFalseUndefined(pub Option<TrueFalse>);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for TrueFalseUndefined {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input
            .try_parse(|input| {
                input
                    .expect_ident_matching("undefined")
                    .map(|()| Self(None))
            })
            .or_else(|_| {
                TrueFalse::parse(input)
                    .map(Some)
                    .map(Self)
                    .map_err(|e| match e {
                        Error::ExpectedIdent { received, .. } => Error::ExpectedIdent {
                            expected: "one of `true` `false` `undefined`",
                            received,
                        },
                        e => e,
                    })
            })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for TrueFalseUndefined {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match &self.0 {
            Some(true_false) => true_false.write_value(dest),
            None => dest.write_str("undefined"),
        }
    }
}
#[test]
fn true_false_undefined() {
    use oxvg_parse::Parse as _;
    assert_eq!(
        TrueFalseUndefined::parse_string("true"),
        Ok(TrueFalseUndefined(Some(TrueFalse(true))))
    );
    assert_eq!(
        TrueFalseUndefined::parse_string("false"),
        Ok(TrueFalseUndefined(Some(TrueFalse(false))))
    );
    assert_eq!(
        TrueFalseUndefined::parse_string("undefined"),
        Ok(TrueFalseUndefined(None))
    );

    assert_eq!(TrueFalseUndefined::parse_string(""), Err(Error::EndOfInput));
    assert_eq!(
        TrueFalseUndefined::parse_string("other"),
        Err(Error::ExpectedIdent {
            expected: "one of `true` `false` `undefined`",
            received: "other"
        })
    );
}

enum_attr!(
    /// Indicates the type of transformation which is to have its values change over time.
    ///
    /// [w3](https://svgwg.org/specs/animations/#AnimateTransformElementTypeAttribute)
    TypeAnimateTransform {
        /// each individual value is expressed as <tx> [,<ty>].
        Translate: "translate",
        /// each individual value is expressed as <sx> [,<sy>].
        Scale: "scale",
        /// each individual value is expressed as <rotate-angle> [<cx> <cy>].
        Rotate: "rotate",
        /// each individual value is expressed as <skew-angle>.
        SkewX: "skewX",
        /// each individual value is expressed as <skew-angle>.
        SkewY: "skewY",
    }
);

#[derive(Clone, Debug, PartialEq)]
/// Specifies a rectangle in user space that should be mapped to the bounds of the SVG viewport.
///
/// [w3](https://svgwg.org/svg2-draft/coords.html#ViewBoxAttribute)
pub struct ViewBox {
    /// The minimum x (i.e. top-most) coordinate of the viewbox
    pub min_x: Number,
    /// The minimum y (i.e. left-most) coordinate of the viewbox
    pub min_y: Number,
    /// The width of the viewbox
    pub width: Number,
    /// The height of the viewbox
    pub height: Number,
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for ViewBox {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input.skip_whitespace();
        let min_x = f32::parse(input)?;
        input.skip_whitespace();
        input.skip_char(',');
        input.skip_whitespace();
        let min_y = f32::parse(input)?;
        input.skip_whitespace();
        input.skip_char(',');
        input.skip_whitespace();
        let width = f32::parse(input)?;
        input.skip_whitespace();
        input.skip_char(',');
        input.skip_whitespace();
        let height = f32::parse(input)?;
        input.skip_whitespace();
        Ok(Self {
            min_x,
            min_y,
            width,
            height,
        })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for ViewBox {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.min_x.write_value(dest)?;
        dest.write_char(' ')?;
        self.min_y.write_value(dest)?;
        dest.write_char(' ')?;
        self.width.write_value(dest)?;
        dest.write_char(' ')?;
        self.height.write_value(dest)
    }
}
#[test]
fn view_box() {
    use oxvg_parse::Parse as _;
    assert_eq!(
        ViewBox::parse_string("1 2 3 4"),
        Ok(ViewBox {
            min_x: 1.0,
            min_y: 2.0,
            width: 3.0,
            height: 4.0
        })
    );

    assert_eq!(ViewBox::parse_string("1 2 3"), Err(Error::InvalidNumber));
    assert_eq!(ViewBox::parse_string("1 2 3 4 5"), Err(Error::ExpectedDone));
}

enum_attr!(
    /// Specifies whether the SVG document can be magnified and panned
    ///
    /// [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/interact.html#ZoomAndPanAttribute)
    ZoomAndPan {
        /// The user agent shall disable any magnification and panning controls
        Disable: "disable",
        /// The user agent shall provide controls
        Magnify: "magnify",
    }
);
