//! Path data attributes as specified in [paths](https://svgwg.org/svg2-draft/paths.html#DProperty)
use std::ops::Deref;

#[cfg(feature = "parse")]
use oxvg_parse::{error::Error, Parse, Parser};
#[cfg(feature = "serialize")]
use oxvg_serialize::{error::PrinterError, Printer, ToValue};

#[derive(Clone, Debug, PartialEq)]
/// A set of path commands
///
/// [w3 | SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/paths.html#PathData)
/// [w3 | SVG 2](https://svgwg.org/svg2-draft/paths.html#DProperty)
pub struct Path(pub oxvg_path::Path);
impl<'input> Parse<'input> for Path {
    // TODO: implement ToCss compatible method
    /// Parse a value from a string
    ///
    /// # Errors
    /// If parsing fails
    fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        oxvg_path::Path::parse(input).map(Self)
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Path {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        use std::fmt::Write;
        struct Writer<W: Write>(W);
        impl<W: Write> Write for Writer<W> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                self.0.write_str(s).map_err(|_| std::fmt::Error)
            }
        }

        write!(Writer(dest), "{}", self.0).map_err(|_| PrinterError {
            kind: lightningcss::error::PrinterErrorKind::FmtError,
            loc: None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Points is equivalent to a [`Path`] starting with an `M` for the first two points,
/// followed by an `L` to subsequent points.
///
/// [SVG 1.1](https://www.w3.org/TR/2011/REC-SVG11-20110816/shapes.html#PointsBNF)
/// [SVG 2](https://svgwg.org/svg2-draft/shapes.html#PolylineElementPointsAttribute)
pub struct Points(pub oxvg_path::Path);
#[cfg(feature = "parse")]
impl<'input> Parse<'input> for Points {
    fn parse(input: &mut Parser<'input>) -> Result<Self, Error<'input>> {
        let mut result = oxvg_path::Path(vec![]);
        result.parse_extend(input, true)?;
        Ok(Self(result))
    }
}
impl Deref for Path {
    type Target = oxvg_path::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[cfg(feature = "serialize")]
impl ToValue for Points {
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        use std::fmt::Write;
        struct Writer<W: Write>(W);
        impl<W: Write> Write for Writer<W> {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                if let Some(s) = s.strip_prefix('M') {
                    self.0.write_str(s).map_err(|_| std::fmt::Error)
                } else {
                    self.0.write_str(s).map_err(|_| std::fmt::Error)
                }
            }
        }

        write!(Writer(dest), "{}", self.0).map_err(|_| PrinterError {
            kind: lightningcss::error::PrinterErrorKind::FmtError,
            loc: None,
        })
    }
}
