//! Parsing for lightningcss values
use lightningcss::{
    properties::{
        display::{Display, Visibility},
        effects::FilterList,
        font::{Font, FontFamily, FontSize, FontStretch, FontStyle, FontWeight},
        masking::{ClipPath, Mask, MaskType},
        overflow::Overflow,
        svg::{
            ColorInterpolation, ColorRendering, ImageRendering, Marker, SVGPaint, ShapeRendering,
            StrokeDasharray, StrokeLinecap, StrokeLinejoin, TextRendering,
        },
        text::{Direction, Spacing, TextDecoration, UnicodeBidi},
        transform::Transform,
        ui::Cursor,
    },
    values::{
        alpha::AlphaValue,
        angle::Angle,
        color::CssColor,
        length::{LengthOrNumber, LengthValue},
        percentage::{DimensionPercentage, Percentage},
        position::{
            HorizontalPositionKeyword, Position, PositionComponent, VerticalPositionKeyword,
        },
        shape::FillRule,
        time::Time,
    },
};

use crate::{error::Error, Parse, Parser};

macro_rules! impl_type {
    ($ty:ty) => {
        impl<'input> Parse<'input> for $ty {
            fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
                <Self as lightningcss::traits::Parse>::parse_string(input.take_slice())
                    .map_err(Error::Lightningcss)
            }
        }
    };
}

impl_type!(AlphaValue);
impl_type!(Angle);
impl_type!(ClipPath<'input>);
impl_type!(ColorInterpolation);
impl_type!(ColorRendering);
impl_type!(CssColor);
impl_type!(Cursor<'input>);
impl_type!(DimensionPercentage<LengthValue>);
impl_type!(Direction);
impl_type!(Display);
impl_type!(FillRule);
impl_type!(FilterList<'input>);
impl_type!(Font<'input>);
impl_type!(FontFamily<'input>);
impl_type!(FontSize);
impl_type!(FontStretch);
impl_type!(FontStyle);
impl_type!(FontWeight);
impl_type!(ImageRendering);
impl_type!(LengthOrNumber);
impl_type!(LengthValue);
impl_type!(Marker<'input>);
impl_type!(Mask<'input>);
impl_type!(MaskType);
impl_type!(Overflow);
impl_type!(Percentage);
impl_type!(Position);
impl_type!(PositionComponent<HorizontalPositionKeyword>);
impl_type!(PositionComponent<VerticalPositionKeyword>);
impl_type!(SVGPaint<'input>);
impl_type!(ShapeRendering);
impl_type!(Spacing);
impl_type!(StrokeDasharray);
impl_type!(StrokeLinecap);
impl_type!(StrokeLinejoin);
impl_type!(TextDecoration);
impl_type!(TextRendering);
impl_type!(Transform);
impl_type!(Time);
impl_type!(UnicodeBidi);
impl_type!(Visibility);
