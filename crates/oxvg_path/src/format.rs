//! Formatting implementations
use crate::{command::Data, paths::positioned, Path};

pub(crate) fn format<'a>(
    mut iter: impl ExactSizeIterator<Item = &'a Data>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    use itertools::Itertools;
    use std::fmt::Display;
    use std::fmt::Write;

    if iter.len() == 1 {
        iter.next().unwrap().fmt(f)?;
        return Ok(());
    }
    iter.tuple_windows()
        .enumerate()
        .try_for_each(|(i, (prev, current))| -> std::fmt::Result {
            if i == 0 {
                prev.fmt(f)?;
            }
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
impl std::fmt::Display for positioned::Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format(self.0.iter().map(|p| &p.command), f)
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
