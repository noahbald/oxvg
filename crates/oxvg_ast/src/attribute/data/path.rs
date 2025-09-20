use std::ops::Deref;

use crate::{
    error::PrinterError,
    serialize::{Printer, ToAtom},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Path(oxvg_path::Path);
impl Path {
    // TODO: implement ToCss compatible method
    pub fn parse_string(definition: &str) -> Result<Self, oxvg_path::parser::Error> {
        oxvg_path::Path::parse_string(definition).map(Self)
    }
}
impl ToAtom for Path {
    fn write_atom<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
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

impl Deref for Path {
    type Target = oxvg_path::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
