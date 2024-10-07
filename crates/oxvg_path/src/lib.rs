//! OXVG Path is a library used for parsing and minifying SVG paths.
//! It supports parsing of any valid SVG path and provides optimisations close to exactly as SVGO.
//!
//! Use the [Path](Path) struct for simple parsing and serializing. By only parsing and serializing,
//! it will produce optimised formatting out of the box.
//! It is made up individual command [Data](command::Data).
//!
//! For more rigorous minification, try using the [run](convert::run) function. This will use
//! non-destructive conversions to shorten the path.
//!
//! # Differences to SVGO
//!
//! - Unlike SVGO, all close paths are serialized as `Z` instead of either `z` or `Z`. This is fine because the two commands function exactly the same.
//! - An equivalent of the `applyTransforms` option isn't available, but may be in the future.
//!
//! # Licensing
//!
//! This library is based off the [`convertPathData`](https://svgo.dev/docs/plugins/convertPathData/) plugin from SVGO and is similarly released under MIT.

#[macro_use]
extern crate bitflags;

pub mod command;
pub mod convert;
pub mod geometry;
pub(crate) mod math;
mod parser;

use command::Position;

use crate::parser::Parser;
use std::fmt::Write;

pub use crate::parser::Error;

#[derive(Debug, Clone, PartialEq)]
/// A path is a set of commands
///
/// # Example
///
/// Out of the box, parsing and serializing a path will produce optimal formatting
///
/// ```
/// use oxvg_path::Path;
///
/// let path = Path::parse("M 10 0.01 L 0.5 -1").unwrap();
/// assert_eq!(&path.to_string(), "M10 .01.5-1");
/// ```
///
/// For more extensive minification, look into using the [run](convert::run) function.
pub struct Path(pub Vec<command::Data>);

#[derive(Debug, Clone)]
/// Equivalent of a [Path](Path), with positional information
pub struct PositionedPath(pub Vec<command::Position>);

impl Path {
    /// Parses a path definition from a string
    ///
    /// # Errors
    /// If the definition is invalid
    pub fn parse(definition: impl Into<String>) -> Result<Self, Error> {
        Parser::default().parse(definition)
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() == 1 {
            self.0.first().unwrap().fmt(f)?;
            return Ok(());
        }
        self.0
            .windows(2)
            .enumerate()
            .try_for_each(|(i, window)| -> std::fmt::Result {
                let prev = &window[0];
                let current = &window[1];
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

impl From<PositionedPath> for Path {
    fn from(value: PositionedPath) -> Self {
        Self(value.0.iter().map(|p| p.command.clone()).collect())
    }
}

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

impl PositionedPath {
    /// Converts self into a [Path](Path), emptying self in the process
    pub fn take(&mut self) -> Path {
        let entries = std::mem::take(&mut self.0);
        Path(entries.into_iter().map(|p| p.command).collect())
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
    pub fn split_mut(path: &mut [Option<Position>], index: usize) -> Option<SplitPositionedPath> {
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
    ) -> Option<SplitPositionedPathWithPrevOption> {
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

impl std::fmt::Display for PositionedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Path(self.0.iter().map(|p| p.command.clone()).collect()).fmt(f)
    }
}

#[test]
fn test_path_parse() {
    // Should parse single command
    insta::assert_snapshot!(dbg!(Path::parse("M 10,50").unwrap()));

    // Should parse multiple commands
    insta::assert_snapshot!(dbg!(Path::parse(
        "M 10,50 C 20,30 40,50 60,70 C 10,20 30,40 50,60"
    )
    .unwrap()));

    // Should parse arc
    insta::assert_snapshot!(dbg!(Path::parse("m-0,1a 25,25 -30 0,1 0,0").unwrap()));

    // Should parse implicit
    insta::assert_snapshot!(dbg!(Path::parse(
        "M 10,50 C 1,2 3,4 5,6.5 .1 .2 .3 .4 .5 -.05176e-005"
    )
    .unwrap()));

    // Should parse minified
    insta::assert_snapshot!(dbg!(
        Path::parse("M10 50C1 2 3 4 5 6.5.1.2.3.4.5-5.176e-7").unwrap()
    ));

    // Should error when command isn't given
    assert!(dbg!(Path::parse("0,0")).is_err());

    // Should error when args are missing
    assert!(dbg!(Path::parse("m1")).is_err());
}
