//! Content types as specified in [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html) and [SVG 2](https://svgwg.org/svg2-draft/propidx.html)
use std::ops::Deref;

use cssparser_lightningcss::Token;
use lightningcss::{
    declaration::DeclarationBlock,
    properties::svg::SVGPaint,
    stylesheet::ParserOptions,
    values::{
        alpha::AlphaValue,
        color::CssColor,
        length::LengthValue,
        number::{CSSInteger, CSSNumber},
    },
};

pub use lightningcss::{
    properties::text::Spacing,
    values::{percentage::Percentage, time::Time},
};

use crate::{
    atom::Atom,
    error::{ParseError, ParseErrorKind, PrinterError},
    parse::{Parse, Parser},
    serialize::{Printer, ToAtom},
};

use super::transform::SVGTransformList;

/// A CSS angle
pub type Angle = lightningcss::values::angle::Angle;
/// A sequence of any characters
pub type Anything<'i> = Atom<'i>;
/// An id string
pub type Id<'i> = Anything<'i>;
/// A class string
pub type Class<'i> = Anything<'i>;

#[derive(Debug, Clone, PartialEq)]
/// A boolean attribute is true when it's empty or matches the attribute's canonical name
///
/// [HTML](https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#boolean-attribute)
pub struct Boolean<'input>(Option<Atom<'input>>);
impl<'input> Parse<'input> for Boolean<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        Ok(Self(
            input
                .try_parse(|input| -> Result<Atom<'input>, ()> {
                    let ident = input.expect_ident().map_err(|_| ())?;
                    Ok(Atom::Cow(ident.into()))
                })
                .ok(),
        ))
    }
}
impl ToAtom for Boolean<'_> {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match &self.0 {
            Some(name) => name.write_atom(dest),
            None => Ok(()),
        }
    }
}

/// A CSS colour
pub type Color = CssColor;

#[derive(Clone, Debug, PartialEq)]
/// A number, absolute, or relative length
pub enum Length {
    /// A number length
    Number(Number),
    /// An absolute length
    Length(LengthValue),
    /// A relative length
    Percentage(Percentage),
}
impl<'input> Parse<'input> for Length {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        input
            .try_parse(Percentage::parse)
            .map(Self::Percentage)
            .or_else(|_| input.try_parse(LengthValue::parse).map(Self::Length))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
impl ToAtom for Length {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_atom(dest),
            Self::Length(length) => length.write_atom(dest),
            Self::Percentage(percentage) => percentage.write_atom(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A frequency in hertz
pub enum Frequency {
    /// Hertz; cycles per second
    Hz(Number),
    /// Kilohertz; cycles per nano-second
    KHz(Number),
}
impl<'input> Parse<'input> for Frequency {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let number = Number::parse(input)?;
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        let str: &str = ident;
        Ok(match str {
            "Hz" => Self::Hz(number),
            "KHz" => Self::KHz(number),
            _ => return Err(location.new_unexpected_token_error(Token::Ident(ident.clone()))),
        })
    }
}
impl ToAtom for Frequency {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Hz(number) => {
                number.write_atom(dest)?;
                dest.write_str("Hz")
            }
            Self::KHz(number) => {
                number.write_atom(dest)?;
                dest.write_str("KHz")
            }
        }
    }
}

/// Functional notation for an IRI
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html#DataTypeFuncIRI)
pub type FuncIRI<'i> = Atom<'i>;
/// An integer
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html#DataTypeInteger)
pub type Integer = CSSInteger;
/// An internationalized resource identifier
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html#DataTypeIRI)
pub type IRI<'i> = Atom<'i>;

/// A non-whitespace, non-parenthesis, non-comma value
pub type Name<'i> = Atom<'i>;
/// A real number
pub type Number = CSSNumber;

#[derive(Clone, Debug, PartialEq)]
/// A pair of numbers, where the second is optional, seperated by a comma or whitespace
pub struct NumberOptionalNumber(pub Number, pub Option<Number>);
impl<'input> Parse<'input> for NumberOptionalNumber {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        let a = Number::parse(input)?;
        input.skip_whitespace();
        let has_comma = input.try_parse(Parser::expect_comma).is_ok();
        input.skip_whitespace();
        let b = if has_comma {
            // Comma makes second number compulsory
            Some(Number::parse(input)?)
        } else {
            input.try_parse(Number::parse).ok()
        };
        Ok(Self(a, b))
    }
}
impl ToAtom for NumberOptionalNumber {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_atom(dest)?;
        if let Some(b) = self.1 {
            dest.write_char(' ')?;
            b.write_atom(dest)?;
        }
        Ok(())
    }
}

/// An alpha value
pub type Opacity = AlphaValue;
/// A paint value
pub type Paint<'i> = SVGPaint<'i>;

#[derive(Clone, Debug, PartialEq)]
/// A CSS style declaration block
pub struct Style<'i>(pub DeclarationBlock<'i>);
impl<'input> Parse<'input> for Style<'input> {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        DeclarationBlock::parse(input, &ParserOptions::default())
            .map(Self)
            .map_err(ParseErrorKind::from_css)
    }
}
impl ToAtom for Style<'_> {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_atom(dest)
    }
}
impl<'input> Deref for Style<'input> {
    type Target = DeclarationBlock<'input>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A raw list of CSS tokens
pub struct TokenList<'input>(pub lightningcss::properties::custom::TokenList<'input>);
impl ToAtom for TokenList<'_> {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        use lightningcss::properties::{
            custom::{CustomProperty, CustomPropertyName},
            Property,
        };
        Property::Custom(CustomProperty {
            name: CustomPropertyName::Unknown("".into()),
            value: self.0.clone(),
        })
        .to_css(dest, false)
    }
}

/// A list of transform values
pub type TransformList = SVGTransformList;
/// A URL string
pub type Url<'i> = Atom<'i>;
