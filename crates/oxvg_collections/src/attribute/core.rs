//! Content types as specified in [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html) and [SVG 2](https://svgwg.org/svg2-draft/propidx.html)
use std::ops::Deref;

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

#[cfg(feature = "parse")]
use oxvg_parse::{error::Error, Parse, Parser};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};

use crate::atom::Atom;

pub use super::transform::SVGTransformList;

/// A CSS angle
pub type Angle = lightningcss::values::angle::Angle;
/// A sequence of any characters
pub type Anything<'i> = Atom<'i>;
/// An id string
pub type Id<'i> = NonWhitespace<'i>;
/// A class string
pub type Class<'i> = NonWhitespace<'i>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A sequence of non-whitespace characters
pub struct NonWhitespace<'i>(pub Anything<'i>);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for NonWhitespace<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input.skip_whitespace();
        let slice = input.take_matches(|char| !char.is_whitespace());
        if slice.is_empty() {
            Err(Error::ExpectedMatch {
                expected: "non-whitespace character",
                received: "nothing",
            })?
        } else {
            Ok(Self(slice.into()))
        }
    }
}
#[cfg(feature = "serialize")]
impl ToValue for NonWhitespace<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_str(&self.0)
    }
}
impl Deref for NonWhitespace<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a> From<&'a str> for NonWhitespace<'a> {
    fn from(value: &'a str) -> Self {
        Self(value.into())
    }
}
#[test]
fn non_whitespace() {
    assert_eq!(
        NonWhitespace::parse_string(" foo "),
        Ok(NonWhitespace("foo".into()))
    );

    assert_eq!(
        NonWhitespace::parse_string(" foo bar "),
        Err(Error::ExpectedDone)
    );
    assert_eq!(
        NonWhitespace::parse_string(""),
        Err(Error::ExpectedMatch {
            expected: "non-whitespace character",
            received: "nothing"
        })
    );
    assert_eq!(
        NonWhitespace::parse_string(" \n\t"),
        Err(Error::ExpectedMatch {
            expected: "non-whitespace character",
            received: "nothing"
        })
    );
}

#[derive(Debug, Clone, PartialEq)]
/// A boolean attribute is true when it's empty or matches the attribute's canonical name
///
/// [HTML](https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#boolean-attribute)
pub struct Boolean<'input>(Option<Atom<'input>>);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Boolean<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input.skip_whitespace();
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
#[cfg(feature = "serialize")]
impl ToValue for Boolean<'_> {
    fn write_value<W>(&self, _dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        // Empty string is valid boolean value
        // https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#boolean-attributes
        Ok(())
    }
}
#[test]
fn boolean() {
    assert_eq!(Boolean::parse_string(""), Ok(Boolean(None)));
    assert_eq!(
        Boolean::parse_string(" autofocus "),
        Ok(Boolean(Some("autofocus".into())))
    );
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
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Length {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        input.skip_whitespace();
        input
            .try_parse(Percentage::parse)
            .map(Self::Percentage)
            .or_else(|_| input.try_parse(LengthValue::parse).map(Self::Length))
            .or_else(|_| Number::parse(input).map(Self::Number))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Length {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Number(number) => number.write_value(dest),
            Self::Length(LengthValue::Px(length)) => length.write_value(dest),
            Self::Length(length) => length.write_value(dest),
            Self::Percentage(percentage) => percentage.write_value(dest),
        }
    }
}
#[test]
fn length() {
    assert_eq!(
        Length::parse_string("20.2350"),
        Ok(Length::Length(LengthValue::Px(20.235)))
    );
    assert_eq!(
        Length::parse_string("20.2268px"),
        Ok(Length::Length(LengthValue::Px(20.2268)))
    );
    assert_eq!(
        Length::parse_string("0.22356em"),
        Ok(Length::Length(LengthValue::Em(0.22356)))
    );
    assert_eq!(
        Length::parse_string("80.0005%"),
        Ok(Length::Percentage(Percentage(0.800_005)))
    );

    assert_eq!(Length::parse_string("20 20"), Err(Error::ExpectedDone));
}

#[derive(Clone, Debug, PartialEq)]
/// A frequency in hertz
pub enum Frequency {
    /// Hertz; cycles per second
    Hz(Number),
    /// Kilohertz; cycles per nano-second
    KHz(Number),
}
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Frequency {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let number = Number::parse(input)?;
        let str = input.expect_ident()?;
        Ok(match str {
            "Hz" => Self::Hz(number),
            "KHz" => Self::KHz(number),
            received => {
                return Err(Error::ExpectedIdent {
                    expected: "one of `Hz` `KHz`",
                    received,
                })
            }
        })
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Frequency {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            Self::Hz(number) => {
                number.write_value(dest)?;
                dest.write_str("Hz")
            }
            Self::KHz(number) => {
                number.write_value(dest)?;
                dest.write_str("KHz")
            }
        }
    }
}
#[test]
fn frequency() {
    assert_eq!(Frequency::parse_string(" 10.5Hz "), Ok(Frequency::Hz(10.5)));
    assert_eq!(Frequency::parse_string(" -1KHz "), Ok(Frequency::KHz(-1.0)));

    assert_eq!(
        Frequency::parse_string("1 Khz"),
        Err(Error::ExpectedIdent {
            expected: "valid ident starting character",
            received: " "
        })
    );
    assert_eq!(
        Frequency::parse_string("1Khz"),
        Err(Error::ExpectedIdent {
            expected: "one of `Hz` `KHz`",
            received: "Khz"
        })
    );
}

/// Functional notation for an IRI
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html#DataTypeFuncIRI)
pub type FuncIRI<'i> = Anything<'i>;
/// An integer
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html#DataTypeInteger)
pub type Integer = CSSInteger;
/// An internationalized resource identifier
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/types.html#DataTypeIRI)
pub type IRI<'i> = Anything<'i>;

/// A non-whitespace, non-parenthesis, non-comma value
pub type Name<'i> = Anything<'i>;
/// A real number
pub type Number = CSSNumber;

#[derive(Clone, Debug, PartialEq)]
/// A pair of numbers, where the second is optional, seperated by a comma or whitespace
pub struct NumberOptionalNumber(pub Number, pub Option<Number>);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for NumberOptionalNumber {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let a = Number::parse(input)?;
        let cursor_before_space = input.cursor();
        input.skip_whitespace();
        let has_comma = input.try_parse(|input| input.expect_char(',')).is_ok();
        input.skip_whitespace();
        let b = if has_comma {
            // Comma makes second number compulsory
            Some(Number::parse(input)?)
        } else {
            let cursor_after_space = input.cursor();
            if let Ok(b) = input.try_parse(Number::parse) {
                if cursor_before_space == cursor_after_space {
                    return Err(Error::ExpectedMatch {
                        expected: "space or comma between numbers",
                        received: "nothing",
                    });
                }
                Some(b)
            } else {
                None
            }
        };
        Ok(Self(a, b))
    }
}
#[cfg(feature = "serialize")]
impl ToValue for NumberOptionalNumber {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_value(dest)?;
        if let Some(b) = self.1 {
            dest.write_char(' ')?;
            b.write_value(dest)?;
        }
        Ok(())
    }
}
#[test]
fn number_optional_number() {
    assert_eq!(
        NumberOptionalNumber::parse_string("10"),
        Ok(NumberOptionalNumber(10.0, None))
    );
    assert_eq!(
        NumberOptionalNumber::parse_string("10 -1"),
        Ok(NumberOptionalNumber(10.0, Some(-1.0)))
    );
    assert_eq!(
        NumberOptionalNumber::parse_string("10,-1"),
        Ok(NumberOptionalNumber(10.0, Some(-1.0)))
    );
    assert_eq!(
        NumberOptionalNumber::parse_string("10 , -1"),
        Ok(NumberOptionalNumber(10.0, Some(-1.0)))
    );
    assert_eq!(
        NumberOptionalNumber::parse_string("10, -1"),
        Ok(NumberOptionalNumber(10.0, Some(-1.0)))
    );

    assert_eq!(
        NumberOptionalNumber::parse_string("10.1.1"),
        Err(Error::ExpectedMatch {
            expected: "space or comma between numbers",
            received: "nothing"
        })
    );
    assert_eq!(
        NumberOptionalNumber::parse_string("10.1-1"),
        Err(Error::ExpectedMatch {
            expected: "space or comma between numbers",
            received: "nothing"
        })
    );
    assert_eq!(
        NumberOptionalNumber::parse_string("10.1 -1 -1"),
        Err(Error::ExpectedDone)
    );
}

/// An alpha value
pub type Opacity = AlphaValue;
/// A paint value
pub type Paint<'i> = SVGPaint<'i>;

#[derive(Clone, Debug, PartialEq)]
/// A CSS style declaration block
pub struct Style<'i>(pub DeclarationBlock<'i>);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Style<'input> {
    fn parse<'t>(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        DeclarationBlock::parse_string(input.take_slice(), ParserOptions::default())
            .map(Self)
            .map_err(Error::Lightningcss)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Style<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.0.write_value(dest)
    }
}
impl<'input> Deref for Style<'input> {
    type Target = DeclarationBlock<'input>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[test]
fn style() {
    assert_eq!(
        Style::parse_string("display: none;"),
        Ok(Style(DeclarationBlock {
            important_declarations: vec![],
            declarations: vec![lightningcss::properties::Property::Display(
                lightningcss::properties::display::Display::Keyword(
                    lightningcss::properties::display::DisplayKeyword::None
                )
            )]
        }))
    );
}

#[derive(Clone, Debug, PartialEq)]
/// A raw list of CSS tokens
pub struct TokenList<'input>(pub lightningcss::properties::custom::TokenList<'input>);
#[cfg(feature = "serialize")]
impl ToValue for TokenList<'_> {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
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

/// A URL string
pub type Url<'i> = Anything<'i>;
