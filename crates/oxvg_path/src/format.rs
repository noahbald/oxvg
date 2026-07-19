//! Formatting implementations
use crate::{command::Data, Path};

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
    let mut prev_args = None;
    iter.tuple_windows()
        .enumerate()
        .try_for_each(|(i, (prev, current))| -> std::fmt::Result {
            if i == 0 {
                f.write_char(prev.id().into())?;
                prev_args = prev.args();
                if let Some(prev_args) = prev_args.as_ref() {
                    prev_args.fmt(f)?;
                }
            }
            let args = current.args();
            if let Some(args) = args.as_ref() {
                if let Some(prev_args) = prev_args.as_ref() {
                    if current.is_implicit() {
                        if args.is_space_needed(prev_args) && *args.first() >= 0.0 {
                            f.write_char(' ')?;
                        }
                    } else {
                        f.write_char(current.id().into())?;
                    }
                } else {
                    f.write_char(current.id().into())?;
                }
                args.fmt(f)?;
            } else {
                f.write_char(current.id().into())?;
            }
            prev_args = args;
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
