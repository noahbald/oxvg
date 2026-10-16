use std::ops::Range;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Span {
    /// Start (inclusive)
    pub start: u32,
    /// End (exclusive)
    pub end: u32,
}

impl Span {
    pub const fn new_usize(start: usize, end: usize) -> Self {
        Self::new(start as u32, end as u32)
    }

    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub const fn splat(at: u32) -> Self {
        Self::new(at, at)
    }

    pub const fn range(range: Range<u32>) -> Self {
        Self::new(range.start, range.end)
    }

    pub fn slice_str(self, source: &str) -> &str {
        &source[self.start as usize..self.end as usize]
    }

    pub fn slice(self, source: &[u8]) -> &[u8] {
        &source[self.start as usize..self.end as usize]
    }
}

pub const SPAN: Span = Span::splat(0);
