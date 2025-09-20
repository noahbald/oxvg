//! XML document representations parsed by different implementations.
//!
//! You can create your own parser to build a tree for OXVG or use one of our
//! implementations for popular parsing libraries.
//! These parsers can be made available with the `"markup5ever"` or `"roxmltree"`
//! feature flags.

use crate::error::{ParseError, ParseErrorKind};

#[cfg(feature = "markup5ever")]
pub mod markup5ever;

#[cfg(feature = "roxmltree")]
pub mod roxmltree;

pub type Parser<'input, 't> = cssparser_lightningcss::Parser<'input, 't>;

pub trait Parse<'input>: Sized {
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>>;

    fn parse_string(input: &'input str) -> Result<Self, ParseError<'input>> {
        let mut input = cssparser_lightningcss::ParserInput::new(input);
        let mut parser = cssparser_lightningcss::Parser::new(&mut input);
        parser.skip_whitespace();
        let result = Self::parse(&mut parser)?;
        parser.expect_exhausted()?;
        Ok(result)
    }
}

impl<'input, T> Parse<'input> for T
where
    T: lightningcss::traits::Parse<'input>,
{
    fn parse<'t>(input: &mut Parser<'input, 't>) -> Result<Self, ParseError<'input>> {
        lightningcss::traits::Parse::parse(input).map_err(|err| {
            cssparser_lightningcss::ParseError {
                kind: match err.kind {
                    cssparser_lightningcss::ParseErrorKind::Custom(err) => {
                        cssparser_lightningcss::ParseErrorKind::Custom(
                            ParseErrorKind::CSSParserError(err),
                        )
                    }
                    cssparser_lightningcss::ParseErrorKind::Basic(err) => {
                        cssparser_lightningcss::ParseErrorKind::Basic(err)
                    }
                },
                location: err.location,
            }
        })
    }
}
