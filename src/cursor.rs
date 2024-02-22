use miette::{SourceOffset, SourceSpan};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Span {
    start: Cursor,
    length: usize,
    source: Option<String>,
}

impl TryInto<SourceSpan> for Span {
    type Error = String;

    fn try_into(self) -> Result<SourceSpan, Self::Error> {
        match self.source {
            Some(s) => Ok((self.start.as_source_offset(s), self.length).into()),
            None => Err("No source found")?,
        }
    }
}

impl Span {
    pub fn as_source_span(&self, source: impl AsRef<str>) -> SourceSpan {
        (self.start.as_source_offset(source), self.length).into()
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Cursor {
    line: usize,
    column: usize,
}

impl Cursor {
    pub fn as_source_offset(&self, source: impl AsRef<str>) -> SourceOffset {
        SourceOffset::from_location(source, self.line + 1, self.column + 1)
    }

    pub fn as_span(&self, length: usize) -> Span {
        Span {
            start: *self,
            length,
            source: None,
        }
    }

    pub fn advance(&self) -> Self {
        Cursor {
            line: self.line,
            column: self.column + 1,
        }
    }

    pub fn mut_advance(&mut self) {
        self.column += 1;
    }

    pub fn advance_by(&self, length: usize) -> Self {
        Cursor {
            line: self.line,
            column: self.column + length,
        }
    }

    pub fn newline(&self) -> Self {
        Cursor {
            line: self.line + 1,
            column: 0,
        }
    }

    pub fn mut_newline(&mut self) {
        self.line += 1;
        self.column = 0;
    }
}
