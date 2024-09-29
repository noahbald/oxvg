use crate::{
    command::{self, Position},
    convert::{self, filter, StyleInfo},
    geometry::Curve,
};

pub fn straight_curve_to_line(
    prev: &Position,
    item: &mut Position,
    next: &mut Option<Position>,
    s_data: &Option<Curve>,
    options: &convert::Options,
    state: &filter::State,
) {
    if !options.flags.straight_curves() {
        return;
    }
    let filter::State { error, saggita, .. } = state;
    match item.command {
        command::Data::CubicBezierBy(ref a) if Curve::is_data_straight(a, *error) => {
            make_specific_longhand(next, &command::ID::SmoothBezierBy, a);
            item.command = command::Data::LineBy([a[4], a[5]]);
        }
        command::Data::SmoothBezierBy(ref a)
            if s_data.as_ref().is_some_and(|s| s.is_straight(*error)) =>
        {
            make_specific_longhand(next, &command::ID::SmoothBezierBy, a);
            item.command = command::Data::LineBy([a[2], a[3]]);
        }
        command::Data::QuadraticBezierBy(ref a) if Curve::is_data_straight(a, *error) => {
            make_specific_longhand(next, &command::ID::SmoothQuadraticBezierBy, a);
            item.command = command::Data::LineBy([a[2], a[3]]);
        }
        command::Data::SmoothQuadraticBezierBy(a)
            if !matches!(
                prev.command,
                command::Data::QuadraticBezierBy(_) | command::Data::SmoothQuadraticBezierBy(_)
            ) =>
        {
            item.command = command::Data::LineBy(a);
        }
        command::Data::ArcBy(a)
            if a[0] == 0.0 || a[1] == 0.0 || saggita.as_ref().is_some_and(|s| s < error) =>
        {
            item.command = command::Data::LineBy([a[5], a[6]]);
        }
        _ => {}
    }
}

fn make_specific_longhand(next: &mut Option<Position>, id: &command::ID, data: &[f64]) {
    let Some(next) = next else {
        return;
    };
    if next.command.id().as_explicit() != id {
        return;
    };
    next.command = next.command.make_longhand(data);
}

pub fn c_to_q(
    item: &mut Position,
    next: &mut Option<Position>,
    options: &convert::Options,
    error: f64,
) {
    if !options.flags.convert_to_q() {
        return;
    }
    let command::Data::CubicBezierBy(args) = item.command else {
        return;
    };
    let Position { start, .. } = item;

    let x1 = 0.75 * (start.0[0] + args[0]) - 0.25 * start.0[0];
    let x2 = 0.75 * (start.0[0] + args[2]) - 0.25 * (start.0[0] + args[4]);
    if f64::abs(x1 - x2) >= error * 2.0 {
        return;
    }

    let y1 = 0.75 * (start.0[1] + args[1]) - 0.25 * start.0[1];
    let y2 = 0.75 * (start.0[1] + args[3]) - 0.25 * (start.0[1] + args[5]);
    if f64::abs(y1 - y2) >= error * 2.0 {
        return;
    }

    let mut new_args = [x1 + x2 - start.0[0], y1 + y2 - start.0[1], args[4], args[5]];
    options.round_data(&mut new_args, error);
    let new_command = command::Data::QuadraticBezierBy(new_args);
    if format!("{new_command}").len() >= format!("{}", item.command).len() {
        return;
    }
    item.command = new_command;

    make_specific_longhand(next, &command::ID::SmoothBezierBy, &args);
}

pub fn line_to_shorthand(item: &mut Position, options: &convert::Options) {
    if !options.flags.line_shorthands() {
        return;
    }
    let command::Data::LineBy(args) = item.command else {
        return;
    };
    if args[1] == 0.0 {
        item.command = command::Data::HorizontalLineBy([args[0]]);
    } else if args[0] == 0.0 {
        item.command = command::Data::VerticalLineBy([args[1]]);
    }
}

pub fn curve_to_shorthand(
    prev: &Position,
    item: &mut Position,
    options: &convert::Options,
    state: &filter::State,
) {
    if !options.flags.curve_smooth_shorthands() {
        return;
    }
    let filter::State {
        error,
        prev_q_control_point,
        ..
    } = state;
    match item.command {
        command::Data::CubicBezierBy(c) => match prev.command {
            // c + c -> c + s
            command::Data::CubicBezierBy(prev_c)
                if f64::abs(c[0] + prev_c[2] - prev_c[4]) < *error
                    && f64::abs(c[1] + prev_c[3] - prev_c[5]) < *error =>
            {
                item.command = command::Data::SmoothBezierBy([c[2], c[3], c[4], c[5]]);
            }
            // s + c -> s + s
            command::Data::SmoothBezierBy(s)
                if f64::abs(c[0] + s[0] - s[2]) < *error
                    && f64::abs(c[1] + s[1] - s[3]) < *error =>
            {
                item.command = command::Data::SmoothBezierBy([c[2], c[3], c[4], c[5]]);
            }
            // ? + c -> ? + s
            _ if c[0].abs() < *error && c[1].abs() < *error => {
                item.command = command::Data::SmoothBezierBy([c[2], c[3], c[4], c[5]]);
            }
            _ => {}
        },
        command::Data::QuadraticBezierBy(q) => match prev.command {
            // q + q -> q + t
            command::Data::QuadraticBezierBy(prev_q)
                if f64::abs(q[0] + prev_q[0] - prev_q[2]) < *error
                    && f64::abs(q[1] + prev_q[1] - prev_q[3]) < *error =>
            {
                item.command = command::Data::SmoothQuadraticBezierBy([q[1], q[2]]);
            }
            // t + q -> t + t
            command::Data::SmoothQuadraticBezierBy(_) => {
                let Some(prev_q_control_point) = prev_q_control_point else {
                    return;
                };
                let predicted_control_point = prev_q_control_point.reflect(item.start);
                let real_control_point = [q[0] + item.start.0[0], q[1] + item.start.0[1]];
                if f64::abs(predicted_control_point.0[0] - real_control_point[0]) >= *error
                    || f64::abs(predicted_control_point.0[1] - real_control_point[1]) >= *error
                {
                    return;
                }
                item.command = command::Data::SmoothQuadraticBezierBy([q[2], q[3]]);
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn home_to_z(
    item: &mut Position,
    next: &mut Option<Position>,
    options: &convert::Options,
    state: &filter::State,
    info: &StyleInfo,
) {
    if !options.flags.convert_to_z() {
        return;
    }
    if !(info.contains(StyleInfo::is_safe_to_use_z)
        || next
            .as_ref()
            .is_some_and(|n| matches!(n.command, command::Data::ClosePath)))
    {
        return;
    }
    if !matches!(
        item.command,
        command::Data::LineBy(_)
            | command::Data::HorizontalLineBy(_)
            | command::Data::VerticalLineBy(_)
    ) {
        return;
    }
    if f64::abs(state.base_path[0] - item.end.0[0]) >= state.error
        || f64::abs(state.base_path[1] - item.end.0[1]) >= state.error
    {
        return;
    }
    item.command = command::Data::ClosePath;
}
