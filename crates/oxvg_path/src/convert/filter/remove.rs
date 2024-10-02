use crate::{
    command::{self, Position},
    convert::{self, filter, StyleInfo},
};

pub fn repeated_close_path(
    prev: &Position,
    item: &Position,
    state: &mut filter::State,
    index: usize,
) -> bool {
    if !matches!(item.command, command::Data::ClosePath) {
        return false;
    }
    state.relative_subpoints[index] = state.base_path;
    if matches!(prev.command, command::Data::ClosePath) {
        return true;
    }
    // prev may not have been `z`, but state is too close to curent position to be considered
    // useful
    state.options.flags.remove_useless()
        && state.info.contains(StyleInfo::is_safe_to_use_z)
        && (item.start.0[0] - item.end.0[0]).abs() < state.error / 10.0
        && (item.start.0[1] - item.end.0[1]).abs() < state.error / 10.0
}

pub fn repeated(
    prev: &mut Position,
    item: &Position,
    options: &convert::Options,
    info: &StyleInfo,
) -> bool {
    let command = &item.command;
    if !options.flags.collapse_repeated()
        || info.contains(StyleInfo::has_marker_mid)
        || !matches!(
            command,
            command::Data::MoveBy(_)
                | command::Data::HorizontalLineBy(_)
                | command::Data::VerticalLineBy(_)
        )
    {
        return false;
    }
    if prev.command.id() != command.id() {
        return false;
    }
    let prev_args = prev.command.args_mut();
    if let command::Data::HorizontalLineBy(a) | command::Data::VerticalLineBy(a) = command {
        // Direction change, e.g negative to positive
        if (prev_args[0] >= 0.0) != (a[0] >= 0.0) {
            return false;
        }
    }

    match command {
        command::Data::HorizontalLineBy(a) | command::Data::VerticalLineBy(a) => {
            prev_args[0] += a[0];
        }
        command::Data::MoveBy(a) => {
            prev_args[0] += a[0];
            prev_args[1] += a[1];
        }
        _ => unreachable!("Conditional guard asserts `h`, `v`, or `m`"),
    }
    prev.end = item.end;
    true
}

pub fn useless_segment(item: &Position, options: &convert::Options, info: &StyleInfo) -> bool {
    let maybe_has_stroke_and_linecap =
        info.contains(StyleInfo::maybe_has_stroke) && info.contains(StyleInfo::maybe_has_linecap);
    if !options.flags.remove_useless() || maybe_has_stroke_and_linecap {
        return false;
    }

    let command = &item.command;
    let all_zero = command
        .args()
        .iter()
        .all(|a| (a - 0.0).abs() < f64::EPSILON);
    if all_zero
        && matches!(
            command,
            command::Data::LineBy(_)
                | command::Data::HorizontalLineBy(_)
                | command::Data::VerticalLineBy(_)
                | command::Data::QuadraticBezierBy(_)
                | command::Data::SmoothQuadraticBezierBy(_)
                | command::Data::CubicBezierBy(_)
                | command::Data::SmoothBezierBy(_)
        )
    {
        return true;
    }

    let command::Data::ArcBy(args) = item.command else {
        return false;
    };
    args[5] == 0.0 && args[6] == 0.0
}
