//! A collection of utility function to filter-map SVG paths.
//!
//! Use the `run` function for a high level way of running all the available conversions to produce
//! the best path optimisation available.
//!
//! From a low-level perspective, the process of optimising a path is as follows:
//! 1. Convert all commands to one type. In our case, we've arbitrarily selected relative commands
//! 2. Filter-map commands by converting, merging, or removing commands when possible
//! 3. Convert commands back to a mix of absolute and relative commands, depending which is more
//!    compressed
//! 4. Cleanup, doing a bit of post-processing to make sure any mistakes made prior are fixed

mod cleanup;
pub mod filter;
mod mixed;
mod relative;

pub use crate::convert::cleanup::{cleanup, cleanup_unpositioned};
pub use crate::convert::filter::filter;
pub use crate::convert::mixed::{mixed, to_absolute};
pub use crate::convert::relative::relative;
use crate::geometry::MakeArcs;
use crate::math::to_fixed;
use crate::{command, Path};

bitflags! {
    /// External style information that may be relevant when optimising a path
    ///
    /// If you aren't able to get such information, try using the
    /// `StyleInfo::conservative` constructor
    #[derive(Debug)]
    pub struct StyleInfo: usize {
        /// Whether a `marker-mid` CSS style is assigned to the element
        const has_marker_mid = 0b0_0001;
        /// Whether a `stroke` style or attribute with an svg-paint is applied to the element
        const maybe_has_stroke = 0b0010;
        /// Whether a `stroke-linecap` style or attribute with `"round"` or `"square` is
        /// applied to the element
        const maybe_has_linecap = 0b100;
        /// Whether a `stroke-linecap` and `stroke-linejoin` style of attribute with `"round"` is
        /// applied to the element
        const is_safe_to_use_z = 0b1000;
        /// Whether a `marker-start` or `marker-end` attribute is applied to the element
        const has_marker = 0b_0001_0000;
    }
}

bitflags! {
    /// Control flags for certain behaviours while optimising a path
    #[derive(Debug)]
    pub struct Flags: usize {
        /// Whether to remove redundant paths that don't draw anything
        const remove_useless_flag= 0b0000_0000_0000_0001;
        /// Whether to round arc radius more accurately
        const smart_arc_rounding_flag= 0b_0000_0000_0010;
        /// Whether to convert commands which are straight into lines
        const straight_curves_flag = 0b00_0000_0000_0100;
        /// Whether to convert cubic beziers to quadratic beziers when they essentially are
        const convert_to_q_flag = 0b_0000_0000_0000_1000;
        /// Whether to convert lines to vertical/horizontal when they move in one direction
        const line_shorthands_flag = 0b00_0000_0001_0000;
        /// Whether to collapse repeated commands which can be expressed as one
        const collapse_repeated_flag = 0b_0000_0010_0000;
        /// Whether to convert smooth curves where possible
        const curve_smooth_shorthands_flag = 0b0100_0000;
        /// Whether to convert returning lines to z
        const convert_to_z_flag = 0b_0000_0000_1000_0000;
        /// Whether to strongly force absolute commands, even when suboptimal
        const force_absolute_path_flag = 0b001_0000_0000;
        /// Whether to weakly force absolute commands, when slightly suboptimal
        const negative_extra_space_flag = 0b10_0000_0000;
        /// Whether to not strongly force relative commands, even when suboptimal
        const utilize_absolute_flag = 0b0_0100_0000_0000;
    }
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Debug, Copy, Clone, Default)]
/// How many decimal points to round path command arguments
pub enum Precision {
    /// Use default precision
    #[default]
    None,
    /// Avoid rounding where possible
    ///
    /// Error tolerance will be 1e-2 where necessary
    Disabled,
    /// Precision to a specific decimal place
    Enabled(i32),
}

#[derive(Debug, Default)]
/// The main options for controlling how the path optimisations are completed.
pub struct Options {
    /// See [`Flags`]
    pub flags: Flags,
    /// See [`MakeArcs`]
    pub make_arcs: MakeArcs,
    /// See [`Precision`]
    pub precision: Precision,
}

/// Returns an optimised version of the input path
///
/// Note that depending on the options and style-info given, the optimisation may be lossy.
///
/// # Examples
///
/// If you don't have any access to attributes or styles for a specific SVG element the path
/// belongs to, try running this with the conservative approach
///
/// ```
/// use oxvg_path::Path;
/// use oxvg_path::convert::{Options, StyleInfo, run};
/// use oxvg_path::parser::Parse as _;
///
/// let mut path = Path::parse_string("M 10,50 L 10,50").unwrap();
/// let options = Options::default();
/// let style_info = StyleInfo::conservative();
///
/// run(&mut path, &options, &style_info);
/// assert_eq!(&path.to_string(), "M10 50h0");
/// ```
pub fn run(path: &mut Path, options: &Options, style_info: &StyleInfo) {
    let includes_vertices = path
        .0
        .iter()
        .any(|c| !matches!(c, command::Data::MoveBy(_) | command::Data::MoveTo(_)));
    // The general optimisation process: original -> naively relative -> filter redundant ->
    // optimal mixed
    log::debug!("convert::run: converting path: {path}");
    let mut positioned_path = relative(std::mem::take(path));
    let mut state = filter::State::new(&positioned_path, options, style_info);
    positioned_path = filter(positioned_path, options, &mut state, style_info);
    if options.flags.utilize_absolute() {
        positioned_path = mixed(positioned_path, options);
    }
    positioned_path = cleanup(positioned_path);
    for command in &mut positioned_path.0 {
        if command.command.is_by() {
            options.round_data(command.command.args_mut(), options.error());
        } else {
            options.round_absolute_command_data(
                command.command.args_mut(),
                options.error(),
                &command.start.0,
            );
        }
    }

    *path = positioned_path.take();
    let has_marker = style_info.contains(StyleInfo::has_marker);
    let is_markers_only_path = has_marker
        && includes_vertices
        && path
            .0
            .iter()
            .all(|c| matches!(c, command::Data::MoveBy(_) | command::Data::MoveTo(_)));
    if is_markers_only_path {
        path.0.push(command::Data::ClosePath);
    }
    log::debug!("convert::run: done: {path}");
}

impl StyleInfo {
    /// Returns a safe set of style-info
    ///
    /// Use this if no contextual details are available
    pub fn conservative() -> Self {
        let mut result = Self::all();
        result.set(Self::is_safe_to_use_z, false);
        result
    }
}

impl Default for StyleInfo {
    fn default() -> Self {
        Self::empty()
    }
}

impl Flags {
    fn remove_useless(&self) -> bool {
        self.contains(Self::remove_useless_flag)
    }

    fn smart_arc_rounding(&self) -> bool {
        self.contains(Self::smart_arc_rounding_flag)
    }

    fn straight_curves(&self) -> bool {
        self.contains(Self::straight_curves_flag)
    }

    fn convert_to_q(&self) -> bool {
        self.contains(Self::convert_to_q_flag)
    }

    fn line_shorthands(&self) -> bool {
        self.contains(Self::line_shorthands_flag)
    }

    fn collapse_repeated(&self) -> bool {
        self.contains(Self::collapse_repeated_flag)
    }

    fn curve_smooth_shorthands(&self) -> bool {
        self.contains(Self::curve_smooth_shorthands_flag)
    }

    fn convert_to_z(&self) -> bool {
        self.contains(Self::convert_to_z_flag)
    }

    fn force_absolute_path(&self) -> bool {
        self.contains(Self::force_absolute_path_flag)
    }

    fn negative_extra_space(&self) -> bool {
        self.contains(Self::negative_extra_space_flag)
    }

    fn utilize_absolute(&self) -> bool {
        self.contains(Self::utilize_absolute_flag)
    }
}

impl Default for Flags {
    fn default() -> Self {
        let mut flags = Self::all();
        flags.set(Self::force_absolute_path_flag, false);
        flags
    }
}

impl Options {
    /// Converts the precision into a tolerance that can be compared against
    pub fn error(&self) -> f64 {
        match self.precision.inner() {
            Some(precision) => {
                let trunc_by = f64::powi(10.0, precision);
                f64::trunc(f64::powi(0.1, precision) * trunc_by) / trunc_by
            }
            None => 1e-2,
        }
    }

    /// Rounds a number to a decimal place based on the given error
    pub fn round(&self, data: f64, error: f64) -> f64 {
        let precision = self.precision.unwrap_or(0);
        if precision > 0 && precision < 20 {
            let fixed = to_fixed(data, precision);
            if (fixed - data).abs() == 0.0 {
                return data;
            }
            let rounded = to_fixed(data, precision - 1);
            if to_fixed((rounded - data).abs(), precision + 1) >= error {
                fixed
            } else {
                rounded
            }
        } else {
            data.round()
        }
    }

    /// Rounds a set of numbers to a decimal place
    pub fn round_data(&self, data: &mut [f64], error: f64) {
        data.iter_mut().enumerate().for_each(|(i, d)| {
            let result = self.round(*d, error);
            if i > 4 && result == 0.0 {
                // Don't accidentally null arcs
                return;
            }
            *d = result;
        });
    }

    /// Rounds a set of numbers to a decimal place
    pub fn round_absolute_command_data(&self, data: &mut [f64], error: f64, start: &[f64; 2]) {
        data.iter_mut().enumerate().for_each(|(i, d)| {
            let result = self.round(*d, error);
            if (i == 5 && result == start[0]) || (i == 6 && result == start[1]) {
                // Don't accidentally null arcs
                return;
            }
            *d = result;
        });
    }

    /// Rounds a path's data to a decimal place
    pub fn round_path(&self, path: &mut Path, error: f64) {
        path.0
            .iter_mut()
            .for_each(|c| self.round_data(c.args_mut(), error));
    }

    /// Produces the safest options for path optimisation
    pub fn conservative() -> Self {
        Self {
            flags: Flags::default(),
            make_arcs: MakeArcs::default(),
            precision: Precision::conservative(),
        }
    }
}

impl Precision {
    fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    fn unwrap_or(self, default: i32) -> i32 {
        match self.inner() {
            Some(x) => x,
            None => default,
        }
    }

    fn inner(self) -> Option<i32> {
        match self {
            Self::Enabled(x) => Some(x),
            Self::None => Some(3),
            Self::Disabled => None,
        }
    }

    /// Returns the maximum possible precision
    pub fn conservative() -> Self {
        Self::Enabled(19)
    }
}

#[test]
fn test_convert() {
    use crate::Path;
    use oxvg_parse::Parse as _;

    let mut path = Path::parse_string("m 1208.23,1821.01 c 74.07,14.24 196.57,17.09 293.43,-14.24 122.5,-42.74 22.79,-199.42 48.43,-207.97 25.64,-8.55 59.83,108.25 287.73,96.86 230.75,-11.39 256.39,-113.95 287.73,-96.86 31.34,17.09 -31.34,284.88 313.37,222.21 0,0 -361.8,96.86 -344.71,-165.23 0,0 -207.96,159.53 -498.54,17.09 2.85,0 76.92,245 -387.44,148.14").unwrap();
    run(&mut path, &Options::default(), &StyleInfo::default());
    assert_eq!(
        String::from(path),
        String::from("M1208.23 1821.01c74.07 14.24 196.57 17.09 293.43-14.24 122.5-42.74 22.79-199.42 48.43-207.97s59.83 108.25 287.73 96.86c230.75-11.39 256.39-113.95 287.73-96.86s-31.34 284.88 313.37 222.21c0 0-361.8 96.86-344.71-165.23 0 0-207.96 159.53-498.54 17.09 2.85 0 76.92 245-387.44 148.14")
    );
}
