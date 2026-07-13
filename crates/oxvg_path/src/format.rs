//! Formatting implementations
use crate::{command::CachedData, Path};

pub(crate) fn format<'a>(
    mut iter: impl ExactSizeIterator<Item = &'a CachedData>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    use itertools::Itertools;
    use std::fmt::Display;
    use std::fmt::Write;

    if iter.len() == 1 {
        iter.next().unwrap().fmt(f)?;
        return Ok(());
    }
    let mut iter = iter.peekable();
    iter.peek().map(|start| start.fmt(f)).transpose()?;
    iter.tuple_windows()
        .try_for_each(|(prev, current)| -> std::fmt::Result {
            let str = current.to_string();
            if current.is_space_needed(prev) && !str.starts_with('-') {
                f.write_char(' ')?;
            }
            f.write_str(&str)?;
            Ok(())
        })
}
impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format(self.0.iter(), f)
    }
}
impl From<Path> for String {
    fn from(value: Path) -> Self {
        format!("{value}")
    }
}

impl From<&Path> for String {
    fn from(value: &Path) -> Self {
        format!("{value}")
    }
}
