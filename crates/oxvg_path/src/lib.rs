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
#[cfg(feature = "optimise")]
#[cfg(feature = "parse")]
#[macro_use]
extern crate bitflags;

#[cfg_attr(feature = "napi", macro_use)]
extern crate napi_derive;

#[cfg(feature = "optimise")]
pub mod command;
#[cfg(feature = "optimise")]
pub mod convert;
#[cfg(feature = "optimise")]
pub mod geometry;
#[cfg(feature = "optimise")]
pub(crate) mod math;
#[cfg(feature = "parse")]
pub mod parser;
#[cfg(feature = "optimise")]
pub mod points;
#[cfg(feature = "optimise")]
pub mod positioned;

use points::{Point, Points};

#[cfg(feature = "parse")]
use crate::parser::Parser;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]

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

impl Path {
    #[cfg(feature = "parse")]
    /// Parses a path definition from a string
    ///
    /// # Errors
    /// If the definition is invalid
    pub fn parse(definition: &str) -> Result<Self, parser::Error> {
        Parser::default().parse(definition)
    }

    /// Checks if two paths have an intersection by checking convex hulls collision using
    /// Gilbert-Johnson-Keerthi distance algorithm.
    ///
    /// # Panics
    /// If internal assertions fail
    pub fn intersects(&self, other: &Self) -> bool {
        let points_1 = Points::from_positioned(&convert::relative(self));
        let points_2 = Points::from_positioned(&convert::relative(other));

        // First check whether their bounding box intersects
        if points_1.max_x <= points_2.min_x
            || points_2.max_x <= points_1.min_x
            || points_1.max_y <= points_2.min_y
            || points_2.max_y <= points_1.min_y
            || points_1.list.iter().all(|set_1| {
                points_2.list.iter().all(|set_2| {
                    set_1.list[set_1.max_x].0[0] <= set_2.list[set_2.min_x].0[0]
                        || set_2.list[set_2.max_x].0[0] <= set_1.list[set_1.min_x].0[0]
                        || set_1.list[set_1.max_y].0[1] <= set_2.list[set_2.min_y].0[1]
                        || set_2.list[set_2.max_y].0[1] <= set_1.list[set_1.min_y].0[1]
                })
            })
        {
            log::debug!("no intersection, bounds check failed");
            return false;
        }

        // i.e. https://en.wikipedia.org/wiki/Gilbert%E2%80%93Johnson%E2%80%93Keerthi_distance_algorithm
        let mut hull_nest_1 = points_1.list.iter().map(Point::convex_hull);
        let hull_nest_2: Vec<_> = points_2.list.iter().map(Point::convex_hull).collect();

        hull_nest_1.any(|hull_1| {
            if hull_1.list.len() < 3 {
                return false;
            }

            hull_nest_2.iter().any(|hull_2| {
                if hull_2.list.len() < 3 {
                    return false;
                }

                let mut simplex = vec![hull_1.get_support(hull_2, geometry::Point([1.0, 0.0]))];
                let mut direction = simplex[0].minus();
                let mut iterations = 10_000;

                loop {
                    iterations -= 1;
                    if iterations == 0 {
                        log::error!("Infinite loop while finding path intersections");
                        return true;
                    }
                    simplex.push(hull_1.get_support(hull_2, direction));
                    if direction.dot(simplex.last().unwrap()) <= 0.0 {
                        return false;
                    }
                    if geometry::Point::process_simplex(&mut simplex, &mut direction) {
                        return true;
                    }
                }
            })
        })
    }
}

#[cfg(feature = "format")]
impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

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

#[cfg(feature = "format")]
impl From<Path> for String {
    fn from(value: Path) -> Self {
        format!("{value}")
    }
}

#[cfg(feature = "format")]
impl From<&Path> for String {
    fn from(value: &Path) -> Self {
        format!("{value}")
    }
}

#[test]
#[cfg(feature = "default")]
fn test_path_parse() {
    // Should parse single command
    insta::assert_snapshot!(Path::parse("M 10,50").unwrap());

    // Should parse multiple commands
    insta::assert_snapshot!(
        Path::parse("M 10,50 C 20,30 40,50 60,70 C 10,20 30,40 50,60").unwrap()
    );

    // Should parse arc
    insta::assert_snapshot!(Path::parse("m-0,1a 25,25 -30 0,1 0,0").unwrap());

    // Should parse implicit
    insta::assert_snapshot!(
        Path::parse("M 10,50 C 1,2 3,4 5,6.5 .1 .2 .3 .4 .5 -.05176e-005").unwrap()
    );

    // Should parse minified
    insta::assert_snapshot!(Path::parse("M10 50C1 2 3 4 5 6.5.1.2.3.4.5-5.176e-7").unwrap());

    // Should error when command isn't given
    assert!(Path::parse("0,0").is_err());

    // Should error when args are missing
    assert!(Path::parse("m1").is_err());
}
