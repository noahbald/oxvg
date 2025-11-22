//! Filter effect attributes as specified in [filter-effects](https://drafts.fxtf.org/filter-effects/)
#[cfg(feature = "parse")]
use oxvg_parse::{error::Error, Parse, Parser};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};

use crate::{atom::Atom, enum_attr};

enum_attr!(
    /// Indicates which channel from `in2` to use to display the pixels in `in` by
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#feDisplacementMapXChannelSelectorAttribute)
    /// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-fedisplacementmap-xchannelselector)
    #[derive(Default)]
    ChannelSelector {
        /// Red
        R: "R",
        /// Green
        G: "G",
        /// Blue
        B: "B",
        /// Alpha
        #[default]
        A: "A",
    }
);
enum_attr!(
    /// Indicates the type of matrix operation
    ///
    /// ```text
    /// | R' |     | r1 r2 r3 r4 r5 |   | R |
    /// | G' |     | g1 g2 g3 g4 g5 |   | G |
    /// | B' |  =  | b1 b2 b3 b4 b5 | * | B |
    /// | A' |     | a1 a2 a3 a4 a5 |   | A |
    /// | 1  |     | 0  0  0  0  1  |   | 1 |
    /// ```
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#feColorMatrixTypeAttribute)
    /// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-fecolormatrix-type)
    #[derive(Default)]
    TypeFeColorMatrix {
        /// Applies a matrix based on `values`
        ///
        /// `values` is a list of 20 matrix values
        #[default]
        Matrix: "matrix",
        /// Creates a saturating matrix based on `values`
        ///
        /// `values` is a real number from 0 to 1
        Saturate: "saturate",
        /// Applies a rotational matrix based on `values`
        ///
        /// `values` is a real number in degrees
        HueRotate: "hueRotate",
        /// Applies a matrix to convert luminence to alpha
        ///
        /// `values` is not applicable
        LuminanceToAlpha: "luminanceToAlpha",
    }
);
enum_attr!(
    /// The compositing operation to be performed
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#feCompositeOperatorAttribute)
    /// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-fecomposite-operator)
    #[derive(Default)]
    OperatorFeComposite {
        /// `in` is painted over `in2`, keeping `in2`
        #[default]
        Over: "over",
        /// `in` is painted inside `in2`'s paint area, omitting `in2`
        In: "in",
        /// `in`  is painted outside `in2`'s paint area, omitting `in2`
        Out: "out",
        /// `in` is painted inside `in2`'s paint area, keeping `in2`
        Atop: "atop",
        /// `in`  is painted outside `in2`'s paint area, omitting `in2` where overlapped
        Xor: "xor",
        /// The sum of `in1` and `in2` is painted where overlapping
        Lighter: "lighter",
        /// Applies the `k1`, `k2`, `k3`, and `k4` attributes to the input of `in` and `in2`
        Arithmetic: "arithmetic",
    }
);
enum_attr!(
    /// Determines how to extend the input image
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#feConvolveMatrixElementEdgeModeAttribute)
    /// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-feconvolvematrix-edgemode)
    #[derive(Default)]
    EdgeMode {
        /// Etend the image at it's edge by duplication
        #[default]
        Duplicate: "duplicate",
        /// Extend the image at it's edges by taking the colours from the opposite edge
        Wrap: "wrap",
        /// Extend the image at it's edge by taking the colours mirrored across the edge
        Mirror: "mirror",
        /// The image is extended with transparent black
        None: "none",
    }
);
enum_attr!(
    /// Specifies whether to erode or dilate
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#feMorphologyOperatorAttribute)
    /// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-femorphology-operator)
    #[derive(Default)]
    OperatorFeMorphology {
        /// Erode (i.e. thin)
        #[default]
        Erode: "erode",
        /// Dilate (i.e. fatten)
        Dilate: "dilate",
    }
);
enum_attr!(
    /// Controls smoothing between turbulence tiles
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#feTurbulenceStitchTilesAttribute)
    /// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-feturbulence-stitchtiles)
    #[derive(Default)]
    StitchTilesFeTurbulence {
        /// Smoothing
        #[default]
        Stitch: "stitch",
        /// No smoothing
        NoStitch: "noStitch",
    }
);
enum_attr!(
    /// Indicates whether the filter primitive should perform noise or turbulence
    ///
    /// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#feTurbulenceTypeAttribute)
    /// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-feturbulence-type)
    #[derive(Default)]
    TypeFeTurbulence {
        /// Fractal noise
        FractalNoise: "fractalNoise",
        /// Turbulence
        #[default]
        Turbulence: "turbulence",
    }
);

#[derive(Clone, Debug, PartialEq, Eq)]
/// Identifies the input for a filter-primitive
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/filters.html#FilterPrimitiveInAttribute)
/// [w3 | SVG 2](https://drafts.fxtf.org/filter-effects/#element-attrdef-filter-primitive-in)
pub enum In<'input> {
    /// The graphics element that was input into the `filter` element
    SourceGraphic,
    /// Like `SourceGraphic` but only the alpha channel is used
    SourceAlpha,
    /// The backdrop behind the filter region of the `filter` element
    BackgroundImage,
    /// Like `BackgroundImage` but only the alpha is used
    BackgroundAlpha,
    /// The `fill` value of the target element
    FillPaint,
    /// The `stroke` value of the target element
    StrokePaint,
    /// The `result` of some preceding element within the `filter` element
    Reference(Atom<'input>),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for In<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let result = input
            .try_parse(|input| {
                let ident: &str = input.expect_ident().map_err(|_| ())?;
                Ok(match ident {
                    "SourceGraphic" => Self::SourceGraphic,
                    "SourceAlpha" => Self::SourceAlpha,
                    "BackgroundImage" => Self::BackgroundImage,
                    "BackgroundAlpha" => Self::BackgroundAlpha,
                    "FillPaint" => Self::FillPaint,
                    "StrokePaint" => Self::StrokePaint,
                    _ => return Err(()),
                })
            })
            .or_else(|()| input.expect_ident().map(Into::into).map(Self::Reference))?;
        input.expect_done()?;
        Ok(result)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for In<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::SourceGraphic => dest.write_str("SourceGraphic"),
            Self::SourceAlpha => dest.write_str("SourceAlpha"),
            Self::BackgroundImage => dest.write_str("BackgroundImage"),
            Self::BackgroundAlpha => dest.write_str("BackgroundAlpha"),
            Self::FillPaint => dest.write_str("FillPaint"),
            Self::StrokePaint => dest.write_str("StrokePaint"),
            Self::Reference(atom) => dest.write_str(atom),
        }
    }
}
#[test]
fn r#in() {
    assert_eq!(In::parse_string("SourceGraphic"), Ok(In::SourceGraphic));
    assert_eq!(In::parse_string("SourceAlpha"), Ok(In::SourceAlpha));
    assert_eq!(In::parse_string("BackgroundImage"), Ok(In::BackgroundImage));
    assert_eq!(In::parse_string("BackgroundAlpha"), Ok(In::BackgroundAlpha));
    assert_eq!(In::parse_string("FillPaint"), Ok(In::FillPaint));
    assert_eq!(In::parse_string("StrokePaint"), Ok(In::StrokePaint));
    assert_eq!(
        In::parse_string(" filter-primitive-reference"),
        Ok(In::Reference("filter-primitive-reference".into()))
    );

    assert_eq!(In::parse_string("trailing "), Err(Error::ExpectedDone));
    assert_eq!(In::parse_string("foo bar"), Err(Error::ExpectedDone));
}
