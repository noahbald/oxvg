//! Path representations with positional information
use crate::command::Position;

#[derive(Debug, Clone)]
/// Equivalent of a [Path](Path), with positional information
pub struct Path(pub Vec<Position>);

type SplitPositionedPath<'a> = (
    &'a mut Position,
    &'a mut Option<Position>,
    &'a mut [Option<Position>],
);

type SplitPositionedPathWithPrevOption<'a> = (
    &'a mut Option<Position>,
    &'a mut Option<Position>,
    &'a mut [Option<Position>],
);

impl Path {
    /// Converts self into a [Path](Path), emptying self in the process
    pub fn take(self) -> crate::Path {
        crate::Path(self.0.into_iter().map(|p| p.command).collect())
    }

    /// Split by `[...prev_paths, prev, item, ...next_paths]`
    ///
    /// # Returns
    /// When the list is of some length, `item` isn't first, and `item` is `Some`
    /// ```ignore
    /// Some(
    ///     // None, if at index 0; otherwise, Some(&mut Option<Position>)
    ///     prev,
    ///     // &mut Some(Position), An item, whose value can be set to None
    ///     item,
    ///     // The rest of the items ahead
    ///     next_paths,
    /// )
    /// ```
    ///
    /// Otherwise, `None`
    pub fn split_mut(
        path: &mut [Option<Position>],
        index: usize,
    ) -> Option<SplitPositionedPath<'_>> {
        let (prev, item, next_paths) = Self::split_mut_with_prev_option(path, index)?;
        let Some(prev) = prev else {
            // Don't change; `item` is first item
            return None;
        };
        Some((prev, item, next_paths))
    }

    /// See `split_mut`
    pub fn split_mut_with_prev_option(
        path: &mut [Option<Position>],
        index: usize,
    ) -> Option<SplitPositionedPathWithPrevOption<'_>> {
        let (prev, next_inclusive) = path.split_at_mut(index);
        let Some((item, next_paths)) = next_inclusive.split_first_mut() else {
            // Can't use; empty list
            return None;
        };
        if item.is_none() {
            // Item already removed
            return None;
        }
        let Some(prev) = prev.iter_mut().rev().find(|p| p.is_some()) else {
            // Don't change; `item` is first item
            return None;
        };
        Some((prev, item, next_paths))
    }
}

impl From<Path> for crate::Path {
    fn from(value: Path) -> Self {
        value.take()
    }
}
