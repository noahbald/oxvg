//! Path data representations for SVG paths
use std::{fmt::Write as _, ops::Deref};

use crate::{geometry::TolerancePrecision, math};

#[derive(Debug, Clone, PartialEq)]
#[allow(
    clippy::unsafe_derive_deserialize,
    reason = "Data::args unrelated to construction"
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
/// Data for a path command
pub enum Data {
    /// M
    /// Move the current point to coordinate `x`, `y`. Any subsequent coordinate pair(s) are
    /// interpreted as parameter(s) for implicit absolute `LineTo` (L) command(s)
    MoveTo([f64; 2]),
    /// m
    MoveBy([f64; 2]),
    /// Z or z
    ClosePath,
    /// L
    LineTo([f64; 2]),
    /// l
    LineBy([f64; 2]),
    /// H
    HorizontalLineTo([f64; 1]),
    /// h
    HorizontalLineBy([f64; 1]),
    /// V
    VerticalLineTo([f64; 1]),
    /// v
    VerticalLineBy([f64; 1]),
    /// C
    CubicBezierTo([f64; 6]),
    /// c
    CubicBezierBy([f64; 6]),
    /// S
    SmoothBezierTo([f64; 4]),
    /// s
    SmoothBezierBy([f64; 4]),
    /// Q
    QuadraticBezierTo([f64; 4]),
    /// q
    QuadraticBezierBy([f64; 4]),
    /// T
    SmoothQuadraticBezierTo([f64; 2]),
    /// t
    SmoothQuadraticBezierBy([f64; 2]),
    /// A
    ArcTo([f64; 7]),
    /// a
    ArcBy([f64; 7]),
    /// An implicit command, which should match the previous command
    Implicit(Box<Data>),
}

#[derive(Clone, Debug, Default, PartialEq)]
/// A type of path command.
pub enum ID {
    /// M
    /// Move the current point to coordinate `x`, `y`. Any subsequent coordinate pair(s) are
    /// interpreted as parameter(s) for implicit absolute `LineTo` (L) command(s)
    MoveTo,
    /// m
    MoveBy,
    /// Z or z
    ClosePath,
    /// L
    LineTo,
    /// l
    LineBy,
    /// H
    HorizontalLineTo,
    /// h
    HorizontalLineBy,
    /// V
    VerticalLineTo,
    /// v
    VerticalLineBy,
    /// C
    CubicBezierTo,
    /// c
    CubicBezierBy,
    /// S
    SmoothBezierTo,
    /// s
    SmoothBezierBy,
    /// Q
    QuadraticBezierTo,
    /// q
    QuadraticBezierBy,
    /// T
    SmoothQuadraticBezierTo,
    /// t
    SmoothQuadraticBezierBy,
    /// A
    ArcTo,
    /// a
    ArcBy,
    /// The absence of any command
    #[default]
    None,
    /// An implicit command, which should match the previous command
    Implicit(Box<ID>),
}

#[derive(Debug)]
/// A container of an SVG command's arguments. This allows debugging and getting first/last
/// values without `None` checking.
pub struct Args<'a>(&'a nunny::Slice<f64>);

pub(crate) struct ProbeLen {
    pub len: usize,
    pub negative: bool,
}

impl Data {
    /// Returns the id for the command
    pub fn id(&self) -> ID {
        match self {
            Self::MoveTo(..) => ID::MoveTo,
            Self::MoveBy(..) => ID::MoveBy,
            Self::ClosePath => ID::ClosePath,
            Self::LineTo(..) => ID::LineTo,
            Self::LineBy(..) => ID::LineBy,
            Self::HorizontalLineTo(..) => ID::HorizontalLineTo,
            Self::HorizontalLineBy(..) => ID::HorizontalLineBy,
            Self::VerticalLineTo(..) => ID::VerticalLineTo,
            Self::VerticalLineBy(..) => ID::VerticalLineBy,
            Self::CubicBezierTo(..) => ID::CubicBezierTo,
            Self::CubicBezierBy(..) => ID::CubicBezierBy,
            Self::SmoothBezierTo(..) => ID::SmoothBezierTo,
            Self::SmoothBezierBy(..) => ID::SmoothBezierBy,
            Self::QuadraticBezierTo(..) => ID::QuadraticBezierTo,
            Self::QuadraticBezierBy(..) => ID::QuadraticBezierBy,
            Self::SmoothQuadraticBezierTo(..) => ID::SmoothQuadraticBezierTo,
            Self::SmoothQuadraticBezierBy(..) => ID::SmoothQuadraticBezierBy,
            Self::ArcTo(..) => ID::ArcTo,
            Self::ArcBy(..) => ID::ArcBy,
            Self::Implicit(command) => ID::Implicit(Box::new(command.id())),
        }
    }

    /// Returns the arguments for the command
    pub fn args(&self) -> Option<Args<'_>> {
        unsafe {
            // SAFETY: All variants are fixed size arrays with len >=1
            Some(Args(nunny::Slice::new_unchecked(match self {
                Self::MoveTo(a)
                | Self::MoveBy(a)
                | Self::LineTo(a)
                | Self::LineBy(a)
                | Self::SmoothQuadraticBezierTo(a)
                | Self::SmoothQuadraticBezierBy(a) => a,
                Self::HorizontalLineTo(a)
                | Self::HorizontalLineBy(a)
                | Self::VerticalLineTo(a)
                | Self::VerticalLineBy(a) => a,
                Self::SmoothBezierTo(a)
                | Self::SmoothBezierBy(a)
                | Self::QuadraticBezierTo(a)
                | Self::QuadraticBezierBy(a) => a,
                Self::CubicBezierTo(a) | Self::CubicBezierBy(a) => a,
                Self::ArcTo(a) | Self::ArcBy(a) => a,
                Self::ClosePath => return None,
                Self::Implicit(a) => return a.args(),
            })))
        }
    }

    /// Returns a mutable reference to the command's arguments
    pub fn args_mut(&mut self) -> &mut [f64] {
        match self {
            Self::MoveTo(a)
            | Self::MoveBy(a)
            | Self::LineTo(a)
            | Self::LineBy(a)
            | Self::SmoothQuadraticBezierTo(a)
            | Self::SmoothQuadraticBezierBy(a) => a,
            Self::ClosePath => &mut [],
            Self::HorizontalLineTo(a)
            | Self::HorizontalLineBy(a)
            | Self::VerticalLineTo(a)
            | Self::VerticalLineBy(a) => a,
            Self::SmoothBezierTo(a)
            | Self::SmoothBezierBy(a)
            | Self::QuadraticBezierTo(a)
            | Self::QuadraticBezierBy(a) => a,
            Self::CubicBezierTo(a) | Self::CubicBezierBy(a) => a,
            Self::ArcTo(a) | Self::ArcBy(a) => a,
            Self::Implicit(a) => a.args_mut(),
        }
    }

    /// Rounds the arguments of the command data up to some precision
    pub fn round(&mut self, precision: TolerancePrecision) {
        let mut nulled = false;
        self.args_mut().iter_mut().enumerate().for_each(|(i, d)| {
            let mut result = precision.round(*d);
            if result == 0.0 {
                match i {
                    7 if nulled => result = precision.descale(1.0),
                    6 => nulled = true,
                    _ => {}
                }
            }
            *d = result;
        });
    }

    /// Set the arg of the command at given index
    ///
    /// # Panics
    /// If the provided index is out of bounds for the type of command
    pub fn set_arg(&mut self, index: usize, value: f64) {
        let args = self.args_mut();
        debug_assert!(
            index < args.len(),
            "Set path command args at out of bounds index"
        );
        args[index] = value;
    }

    /// Returns whether the command is implicit
    pub fn is_implicit(&self) -> bool {
        matches!(self, Self::Implicit(_))
    }

    /// Returns the command, converting from implicit if necessary
    pub fn as_explicit(&self) -> &Self {
        if let Self::Implicit(inner) = self {
            return inner.as_explicit();
        }
        self
    }

    /// Returns whether the command goes to an absolute position.
    pub fn is_to(&self) -> bool {
        match self {
            Self::MoveTo(_)
            | Self::ClosePath
            | Self::LineTo(_)
            | Self::HorizontalLineTo(_)
            | Self::VerticalLineTo(_)
            | Self::CubicBezierTo(_)
            | Self::SmoothBezierTo(_)
            | Self::QuadraticBezierTo(_)
            | Self::SmoothQuadraticBezierTo(_)
            | Self::ArcTo(_) => true,
            Self::Implicit(c) => c.is_to(),
            _ => false,
        }
    }

    /// Returns whether the command goes to a relative position.
    pub fn is_by(&self) -> bool {
        matches!(self, Self::ClosePath) || !self.is_to()
    }

    /// Calculates the saggita of an arc-by if possible
    pub fn calculate_saggita(&self, error: f64) -> Option<f64> {
        let Self::ArcBy(args) = self else {
            return None;
        };
        math::saggita(args, error)
    }

    pub(crate) fn size_hint_with_args(&self, args: Option<&Args>) -> ProbeLen {
        let negative = self.is_implicit()
            && args.is_some_and(|args| {
                let a = args.first();
                a.is_sign_negative() && *a != -0.0
            });
        let mut count = ProbeLen { len: 0, negative };
        let _ = write!(count, "{self}");
        count
    }
}

impl std::fmt::Write for ProbeLen {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.len += s.len();
        Ok(())
    }
}

impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id().fmt(f)?;
        if let Some(args) = self.args() {
            args.fmt(f)
        } else {
            Ok(())
        }
    }
}

impl From<(&ID, [f64; 7])> for Data {
    fn from(value: (&ID, [f64; 7])) -> Self {
        let (command_id, args) = value;
        match command_id {
            ID::MoveTo => Self::MoveTo([args[0], args[1]]),
            ID::MoveBy => Self::MoveBy([args[0], args[1]]),
            ID::ClosePath => Self::ClosePath,
            ID::LineTo => Self::LineTo([args[0], args[1]]),
            ID::LineBy => Self::LineBy([args[0], args[1]]),
            ID::HorizontalLineTo => Self::HorizontalLineTo([args[0]]),
            ID::HorizontalLineBy => Self::HorizontalLineBy([args[0]]),
            ID::VerticalLineTo => Self::VerticalLineTo([args[0]]),
            ID::VerticalLineBy => Self::VerticalLineBy([args[0]]),
            ID::CubicBezierTo => {
                Self::CubicBezierTo([args[0], args[1], args[2], args[3], args[4], args[5]])
            }
            ID::CubicBezierBy => {
                Self::CubicBezierBy([args[0], args[1], args[2], args[3], args[4], args[5]])
            }
            ID::SmoothBezierTo => Self::SmoothBezierTo([args[0], args[1], args[2], args[3]]),
            ID::SmoothBezierBy => Self::SmoothBezierBy([args[0], args[1], args[2], args[3]]),
            ID::QuadraticBezierTo => Self::QuadraticBezierTo([args[0], args[1], args[2], args[3]]),
            ID::QuadraticBezierBy => Self::QuadraticBezierBy([args[0], args[1], args[2], args[3]]),
            ID::SmoothQuadraticBezierTo => Self::SmoothQuadraticBezierTo([args[0], args[1]]),
            ID::SmoothQuadraticBezierBy => Self::SmoothQuadraticBezierBy([args[0], args[1]]),
            ID::ArcTo => Self::ArcTo([
                args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            ]),
            ID::ArcBy => Self::ArcBy([
                args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            ]),
            ID::None => unreachable!(),
            ID::Implicit(command) => Data::Implicit(Box::new(Data::from((command.as_ref(), args)))),
        }
    }
}

impl ID {
    /// Returns the length of a command's arguments
    pub fn args(&self) -> usize {
        match self {
            Self::ClosePath | Self::None => 0,
            Self::HorizontalLineTo
            | Self::HorizontalLineBy
            | Self::VerticalLineTo
            | Self::VerticalLineBy => 1,
            Self::LineTo
            | Self::LineBy
            | Self::MoveTo
            | Self::MoveBy
            | Self::SmoothQuadraticBezierTo
            | Self::SmoothQuadraticBezierBy => 2,
            Self::SmoothBezierTo
            | Self::SmoothBezierBy
            | Self::QuadraticBezierTo
            | Self::QuadraticBezierBy => 4,
            Self::CubicBezierTo | Self::CubicBezierBy => 6,
            Self::ArcTo | Self::ArcBy => 7,
            Self::Implicit(command) => command.args(),
        }
    }

    /// Returns whether the command is `None`, i.e. a non-representable command.
    ///
    /// This may be used to represent a command that couldn't/hasn't been parsed.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns whether the command is implicit, based on the previous command.
    pub fn is_implicit(&self) -> bool {
        matches!(self, Self::Implicit(_))
    }

    /// Converts the command if it's implicit
    pub fn as_explicit(&self) -> &Self {
        if let Self::Implicit(inner) = self {
            return inner.as_explicit();
        }
        self
    }

    #[must_use]
    /// Returns the expected command to follow this one if it's implicit
    pub fn next_implicit(&self) -> Self {
        match self {
            Self::MoveTo => Self::LineTo,
            Self::MoveBy => Self::LineBy,
            Self::Implicit(c) => c.next_implicit(),
            c => c.clone(),
        }
    }
}

impl TryFrom<char> for ID {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'M' => Ok(Self::MoveTo),
            'm' => Ok(Self::MoveBy),
            'L' => Ok(Self::LineTo),
            'l' => Ok(Self::LineBy),
            'H' => Ok(Self::HorizontalLineTo),
            'h' => Ok(Self::HorizontalLineBy),
            'V' => Ok(Self::VerticalLineTo),
            'v' => Ok(Self::VerticalLineBy),
            'C' => Ok(Self::CubicBezierTo),
            'c' => Ok(Self::CubicBezierBy),
            'S' => Ok(Self::SmoothBezierTo),
            's' => Ok(Self::SmoothBezierBy),
            'Q' => Ok(Self::QuadraticBezierTo),
            'q' => Ok(Self::QuadraticBezierBy),
            'T' => Ok(Self::SmoothQuadraticBezierTo),
            't' => Ok(Self::SmoothQuadraticBezierBy),
            'A' => Ok(Self::ArcTo),
            'a' => Ok(Self::ArcBy),
            'Z' | 'z' => Ok(Self::ClosePath),
            _ => Err(()),
        }
    }
}

impl From<&ID> for char {
    fn from(value: &ID) -> Self {
        match value {
            ID::MoveTo => 'M',
            ID::MoveBy => 'm',
            ID::ClosePath => 'Z',
            ID::LineTo => 'L',
            ID::LineBy => 'l',
            ID::HorizontalLineTo => 'H',
            ID::HorizontalLineBy => 'h',
            ID::VerticalLineTo => 'V',
            ID::VerticalLineBy => 'v',
            ID::CubicBezierTo => 'C',
            ID::CubicBezierBy => 'c',
            ID::SmoothBezierTo => 'S',
            ID::SmoothBezierBy => 's',
            ID::QuadraticBezierTo => 'Q',
            ID::QuadraticBezierBy => 'q',
            ID::SmoothQuadraticBezierTo => 'T',
            ID::SmoothQuadraticBezierBy => 't',
            ID::ArcTo => 'A',
            ID::ArcBy => 'a',
            ID::None => unreachable!(),
            ID::Implicit(_) => ' ',
        }
    }
}

impl From<ID> for char {
    fn from(value: ID) -> Self {
        (&value).into()
    }
}

impl std::fmt::Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_implicit() {
            Ok(())
        } else {
            f.write_char(self.into())
        }
    }
}

impl<'a> Deref for Args<'a> {
    type Target = &'a nunny::Slice<f64>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Args<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = ryu::Buffer::new();
        let mut previous_option = None;
        self.0.iter().try_for_each(|current| -> std::fmt::Result {
            let Some(previous) = previous_option else {
                previous_option = Some(*current);
                return short_number_from_buffer(buffer.format(*current), f);
            };
            let current = *current;
            let raw = buffer.format(current);
            #[allow(clippy::float_cmp)] // This is fine for formatting
            if current >= 1.0
                || (current == 0.0)
                || (previous == 0.0 && current >= 0.0)
                || (previous % 1.0 == 0.0 && raw.starts_with("0."))
                || (current > 0.0 && current < 1e-4)
            {
                f.write_char(' ')?;
            }
            previous_option = Some(current);
            short_number_from_buffer(raw, f)
        })
    }
}

impl Args<'_> {
    /// Whether, when formatting itself, a space is needed between itself and the previous
    /// command.
    /// This check is only needed when `self` is for an implicit command.
    pub(crate) fn is_space_needed(&self, prev: &Self) -> bool {
        (prev.last() % 1.0) == 0.0 || self.first() >= &1.0 || self.first() == &0.0
    }
}

pub(crate) fn short_number_from_buffer<W: std::fmt::Write>(
    mut raw: &str,
    w: &mut W,
) -> std::fmt::Result {
    // Remove trailing zeros
    if raw.contains('.') {
        raw = raw.strip_suffix('0').unwrap_or(raw);
    }
    if matches!(raw, "0." | "-0.") {
        return w.write_char('0');
    }
    raw = raw.strip_suffix('.').unwrap_or(raw);
    // Remove leading zero
    if raw.starts_with("0.") {
        w.write_str(&raw[1..])
    } else if raw.starts_with("-0.") {
        w.write_char('-')?;
        w.write_str(&raw[2..])
    } else {
        w.write_str(raw)
    }
}

/// Formats a command's argument into it's shortest possible form
pub fn short_number<F>(n: F) -> String
where
    F: ryu::Float,
{
    let mut buffer = ryu::Buffer::new();
    let raw = buffer.format(n);
    let mut output = String::with_capacity(raw.len());
    let _ = short_number_from_buffer(raw, &mut output);
    output
}
