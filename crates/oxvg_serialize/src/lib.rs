//! Primitives for serializing XML values

use error::PrinterError;

pub mod error;

/// The destination to output serialized attribute and CSS values
pub type Printer<'a, 'b, 'c, W> = lightningcss::printer::Printer<'a, 'b, 'c, W>;
/// Options that control how attributes and CSS values are serialized
pub type PrinterOptions<'a> = lightningcss::printer::PrinterOptions<'a>;

/// Trait for values that can be serialized into string-like formats
pub trait ToValue {
    /// Serialize `self` into SVG content, writing to `dest`
    ///
    /// # Errors
    /// If printer fails
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write;

    /// Serialize `self` into SVG content and return a string
    ///
    /// # Errors
    /// If writing string fails
    fn to_value_string(&self, options: PrinterOptions) -> Result<String, PrinterError> {
        let mut s = String::new();
        let mut printer = Printer::new(&mut s, options);
        self.write_value(&mut printer)?;
        Ok(s)
    }
}

impl<T> ToValue for T
where
    T: lightningcss::traits::ToCss,
{
    fn write_value<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
    where
        W: std::fmt::Write,
    {
        self.to_css(dest)
    }
}
