use cssparser_lightningcss::Token;
use lightningcss::media_query::MediaList;

use crate::{
    atom::Atom,
    enum_attr,
    error::{ParseError, ParseErrorKind, PrinterError},
    parse::Parser,
    serialize::{Printer, ToAtom},
};

use super::{
    core::{Angle, Number, Percentage},
    presentation::{LengthOrNumber, LengthPercentage},
    transform::SVGTransform,
    Parse,
};

enum_attr!(BlendMode {
    Normal: "normal",
    Darken: "darken",
    Multiply: "multiply",
    ColorBurn: "color-burn",
    Lighten: "lighten",
    Screen: "screen",
    ColorDodge: "color-dodge",
    Overlay: "overlay",
    SoftLight: "soft-light",
    HardLight: "hard-light",
    Difference: "difference",
    Exclusion: "exclusion",
    Hue: "hue",
    Saturation: "saturation",
    Color: "color",
    Luminosity: "luminosity",
});
enum_attr!(CrossOrigin {
    Anonymous: "anonymous",
    UseCredentials: "use-credentials",
});
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreserveAspectRatio {
    align: PreserveAspectRatioAlign,
    meet_or_slice: Option<PreserveAspectRatioMeetOrSlice>,
}
impl<'input> Parse<'input> for PreserveAspectRatio {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        let align = PreserveAspectRatioAlign::parse(input)?;
        input.skip_whitespace();
        Ok(Self {
            align,
            meet_or_slice: input
                .try_parse(|input| PreserveAspectRatioMeetOrSlice::parse(input))
                .ok(),
        })
    }
}
impl ToAtom for PreserveAspectRatio {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.align.write_atom(dest)?;
        if let Some(meet_or_slice) = &self.meet_or_slice {
            dest.write_char(' ')?;
            meet_or_slice.write_atom(dest)?;
        }
        Ok(())
    }
}

enum_attr!(PreserveAspectRatioAlign {
    None: "none",
    XMinYMin: "xMinYMin",
    XMidYMin: "xMidYMin",
    XMaxYMin: "xMaxYMin",
    XMinYMid: "xMinYMid",
    XMidYMid: "xMidYMid",
    XMaxYMid: "xMaxYMid",
    XMinYMax: "xMinYMax",
    XMidYMax: "xMidYMax",
    XMaxYMax: "xMaxYMax",
});

enum_attr!(PreserveAspectRatioMeetOrSlice {
    Meet: "meet",
    Slice: "slice",
});

enum_attr!(Units {
    UserSpaceOnUse: "userSpaceOnUse",
    ObjectBoundingBox: "objectBoundingBox",
});

enum_attr!(LengthAdjust {
    Spacing: "spacing",
    SpacingAndGlyphs: "spacingAndGlyphs",
});

enum_attr!(LinkType {
    Alternate: "alternate",
    Author: "author",
    Bookmark: "bookmark",
    Canonical: "canonical",
    CompressionDictionary: "compression-dictionary",
    DnsPrefetch: "dns-prefetch",
    External: "external",
    Expect: "expect",
    Help: "help",
    Icon: "icon",
    License: "license",
    Manifest: "manifest",
    Me: "me",
    Modulepreload: "modulepreload",
    Next: "next",
    Nofollow: "nofollow",
    Noreferrer: "noreferrer",
    Opener: "opener",
    Pingback: "pingback",
    Preconnect: "preconnect",
    Prefetch: "prefetch",
    Preload: "preload",
    Prerender: "prerender",
    Prev: "prev",
    PrivacyPolicy: "privacy-policy",
    Search: "search",
    Stylesheet: "stylesheet",
    Tag: "tag",
    TermsOfService: "terms-of-service",
});

pub type MediaType<'i> = Atom<'i>;

#[derive(Debug, Clone, PartialEq)]
pub struct MediaQueryList<'i>(MediaList<'i>);
impl<'input> Parse<'input> for MediaQueryList<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        MediaList::parse(input)
            .map(Self)
            .map_err(ParseErrorKind::from_css)
    }
}
impl<'input> ToAtom for MediaQueryList<'input> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_atom(dest)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumberPercentage {
    Number(Number),
    Percentage(Percentage),
}
impl<'input> Parse<'input> for NumberPercentage {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| Percentage::parse(input).map(Self::Percentage))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
impl ToAtom for NumberPercentage {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_atom(dest),
            Self::Percentage(percentage) => percentage.write_atom(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Orient {
    Auto,
    AutoStartReverse,
    Angle(Angle),
    Number(Number),
}
impl<'input> Parse<'input> for Orient {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                input.expect_ident().map_err(|_| ()).and_then(|ident| {
                    let ident: &str = &*ident;
                    Ok(match ident {
                        "auto" => Self::Auto,
                        "auto-start-reverse" => Self::AutoStartReverse,
                        _ => return Err(()),
                    })
                })
            })
            .or_else(|_| input.try_parse(|input| Angle::parse(input).map(Self::Angle)))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
impl ToAtom for Orient {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::AutoStartReverse => dest.write_str("auto-start-reverse"),
            Self::Angle(angle) => angle.write_atom(dest),
            Self::Number(number) => number.write_atom(dest),
        }
    }
}

enum_attr!(Origin { Default: "default" });

#[derive(Clone, Debug, PartialEq)]
pub enum Radius {
    LengthPercentage(LengthPercentage),
    Auto,
}
impl<'input> Parse<'input> for Radius {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(
                |input| -> Result<Self, cssparser_lightningcss::BasicParseError> {
                    input.expect_ident_matching("auto")?;
                    Ok(Self::Auto)
                },
            )
            .or_else(|_| LengthPercentage::parse(input).map(Self::LengthPercentage))
    }
}
impl ToAtom for Radius {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::LengthPercentage(v) => v.write_atom(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RefX {
    LengthOrNumber(LengthOrNumber),
    Left,
    Center,
    Right,
}
impl<'input> Parse<'input> for RefX {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                Ok(match ident {
                    "left" => Self::Left,
                    "center" => Self::Center,
                    "right" => Self::Right,
                    _ => return Err(()),
                })
            })
            .or_else(|_| {
                input.skip_whitespace();
                LengthOrNumber::parse(input).map(Self::LengthOrNumber)
            })
    }
}
impl ToAtom for RefX {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::LengthOrNumber(length_or_number) => length_or_number.write_atom(dest),
            Self::Left => dest.write_str("left"),
            Self::Center => dest.write_str("center"),
            Self::Right => dest.write_str("right"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RefY {
    LengthOrNumber(LengthOrNumber),
    Top,
    Center,
    Bottom,
}
impl<'input> Parse<'input> for RefY {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                Ok(match ident {
                    "top" => Self::Top,
                    "center" => Self::Center,
                    "bottom" => Self::Bottom,
                    _ => return Err(()),
                })
            })
            .or_else(|_| {
                input.skip_whitespace();
                LengthOrNumber::parse(input).map(Self::LengthOrNumber)
            })
    }
}
impl ToAtom for RefY {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::LengthOrNumber(length_or_number) => length_or_number.write_atom(dest),
            Self::Top => dest.write_str("top"),
            Self::Center => dest.write_str("center"),
            Self::Bottom => dest.write_str("bottom"),
        }
    }
}

enum_attr!(ReferrerPolicy {
    NoReferrer: "no-referrer",
    NoReferrerWhenDowngrade: "no-referrer-when-downgrade",
    SameOrigin: "same-origin",
    Origin: "origin",
    StrictOrigin: "strict-origin",
    OriginWhenCrossOrigin: "origin-when-cross-origin",
    StrictOriginWhenCrossOrigin: "strict-origin-when-cross-origin",
    UnsafeUrl: "unsafe-url",
});

#[derive(Clone, Debug, PartialEq)]
pub enum Rotate {
    Number(Number),
    Auto,
    AutoReverse,
}
impl<'input> Parse<'input> for Rotate {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                Ok(match ident {
                    "auto" => Self::Auto,
                    "auto-reverse" => Self::AutoReverse,
                    _ => return Err(()),
                })
            })
            .or_else(|_| {
                input.skip_whitespace();
                Number::parse(input).map(Self::Number)
            })
    }
}
impl ToAtom for Rotate {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Auto => dest.write_str("auto"),
            Self::AutoReverse => dest.write_str("auto-reverse"),
            Self::Number(number) => number.write_atom(dest),
        }
    }
}

enum_attr!(SpreadMethod {
    Pad: "pad",
    Reflect: "reflect",
    Repeat: "repeat",
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target<'input> {
    _Self,
    _Parent,
    _Top,
    _Blank,
    XMLName(Atom<'input>),
}
impl<'input> Parse<'input> for Target<'input> {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        Ok(input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
                Ok(match &*ident {
                    "_self" => Self::_Self,
                    "_parent" => Self::_Parent,
                    "_top" => Self::_Top,
                    "_blank" => Self::_Blank,
                    _ => return Err(()),
                })
            })
            .unwrap_or_else(|_| Self::XMLName(input.slice_from(input.position()).into())))
    }
}
impl ToAtom for Target<'_> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::_Self => dest.write_str("_self"),
            Self::_Parent => dest.write_str("_parent"),
            Self::_Top => dest.write_str("_top"),
            Self::_Blank => dest.write_str("_blank"),
            Self::XMLName(name) => name.write_atom(dest),
        }
    }
}

enum_attr!(TextPathMethod {
    Align: "align",
    Stretch: "stretch",
});

enum_attr!(TextPathSpacing {
    Auto: "auto",
    Exact: "exact",
});

enum_attr!(TextPathSide {
    Left: "left",
    Right: "right",
});

pub type Transform = SVGTransform;

#[derive(Clone, Debug, PartialEq)]
pub struct TrueFalse(bool);
impl<'input> Parse<'input> for TrueFalse {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        let str: &str = &*ident;
        Ok(Self(match str {
            "true" => true,
            "false" => false,
            _ => return Err(location.new_unexpected_token_error(Token::Ident(ident.clone()))),
        }))
    }
}
impl ToAtom for TrueFalse {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
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

#[derive(Clone, Debug, PartialEq)]
pub struct TrueFalseUndefined(Option<TrueFalse>);
impl<'input> Parse<'input> for TrueFalseUndefined {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(|input| input.expect_ident_matching("undefined").map(|_| Self(None)))
            .or_else(|_| TrueFalse::parse(input).map(Some).map(Self))
    }
}
impl ToAtom for TrueFalseUndefined {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match &self.0 {
            Some(true_false) => true_false.write_atom(dest),
            None => dest.write_str("undefined"),
        }
    }
}

enum_attr!(TypeAnimateTransform {
    Translate: "translate",
    Scale: "scale",
    Rotate: "rotate",
    SkewX: "skewX",
    SkewY: "skewY",
});

#[derive(Clone, Debug, PartialEq)]
pub struct ViewBox {
    min_x: Number,
    min_y: Number,
    width: Number,
    height: Number,
}
impl<'input> Parse<'input> for ViewBox {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input.skip_whitespace();
        let min_x = input.expect_number()?;
        input.skip_whitespace();
        input.try_parse(cssparser_lightningcss::Parser::expect_comma);
        input.skip_whitespace();
        let min_y = input.expect_number()?;
        input.skip_whitespace();
        input.try_parse(cssparser_lightningcss::Parser::expect_comma);
        input.skip_whitespace();
        let width = input.expect_number()?;
        input.skip_whitespace();
        input.try_parse(cssparser_lightningcss::Parser::expect_comma);
        input.skip_whitespace();
        let height = input.expect_number()?;
        input.skip_whitespace();
        Ok(Self {
            min_x,
            min_y,
            width,
            height,
        })
    }
}
impl ToAtom for ViewBox {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.min_x.write_atom(dest)?;
        dest.write_char(' ')?;
        self.min_y.write_atom(dest)?;
        dest.write_char(' ')?;
        self.width.write_atom(dest)?;
        dest.write_char(' ');
        self.height.write_atom(dest)
    }
}
