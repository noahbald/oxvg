use crate::{
    atom::Atom,
    enum_attr,
    error::{ParseErrorKind, PrinterError},
    parse::Parse,
    serialize::{Printer, ToAtom},
};

enum_attr!(ChannelSelector {
    R: "R",
    G: "G",
    B: "B",
    A: "A",
});
enum_attr!(FeColorMatrixType {
    Matrix: "matrix",
    Saturate: "saturate",
    HueRotate: "hueRotate",
    LuminanceToAlpha: "luminanceToAlpha",
});
enum_attr!(FeCompositeOperator {
    Over: "over",
    In: "in",
    Out: "out",
    Atop: "atop",
    Xor: "xor",
    Lighter: "lighter",
    Arithmetic: "arithmetic",
});
enum_attr!(FeEdgeMode {
    Duplicate: "duplicate",
    Wrap: "wrap",
    None: "none",
});
enum_attr!(FeOperator {
    Erode: "erode",
    Dilate: "dilate",
});
enum_attr!(FeTurbulenceStitchTiles {
    Stitch: "stitch",
    NoStitch: "noStitch",
});
enum_attr!(FeTurbulenceType {
    FractalNoise: "fractalNoise",
    Turbulence: "turbulence",
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum In<'input> {
    SourceGraphic,
    SourceAlpha,
    BackgroundImage,
    BackgroundAlpha,
    FillPaint,
    StrokePaint,
    Reference(Atom<'input>),
}
impl<'input> Parse<'input> for In<'input> {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<Self, cssparser_lightningcss::ParseError<'input, ParseErrorKind<'input>>> {
        input
            .try_parse(|input| {
                let ident: &str = &*(input.expect_ident().map_err(|_| ())?);
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
            .or_else(|_| Atom::parse(input).map(Self::Reference))
    }
}
impl<'input> ToAtom for In<'input> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
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
            Self::Reference(atom) => dest.write_str(&atom),
        }
    }
}
