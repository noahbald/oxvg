//! Definitions for the commands of path data.
use crate::{
    command::Data,
    geometry::{Curve, Point},
};

#[derive(Debug, Clone)]
/// The equivalent of a [Path](crate::Path), but with additional positional information
pub struct Position {
    /// The path command.
    pub command: Data,
    /// The base point of the command
    pub start: Point,
    /// The coords the the command goes to
    pub end: Point,
    /// If available, the equivalent [`SmoothBezierBy`](crate::command::Data::SmoothBezierBy) args
    pub s_data: Option<Curve>,
}
