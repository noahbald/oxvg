use crate::{
    command::{self, Position},
    convert::{self, filter},
    math,
};

pub fn relative_coordinates(
    item: &mut Position,
    state: &mut filter::State,
    options: &convert::Options,
    index: usize,
) {
    if options.precision.is_disabled() {
        return;
    }
    update_relative_subpoint(item, options, state, index);
    options.round_data(item.command.args_mut(), state.error);

    match item.command {
        command::Data::HorizontalLineBy(a) => {
            let relative_subpoint = &mut state.relative_subpoints[index];
            relative_subpoint[0] += a[0];
        }
        command::Data::VerticalLineBy(a) => {
            let relative_subpoint = &mut state.relative_subpoints[index];
            relative_subpoint[1] += a[0];
        }
        _ => {
            let relative_subpoint = &mut state.relative_subpoints[index];
            let a = item.command.args();
            if a.len() < 2 {
                return;
            }
            relative_subpoint[0] += a[a.len() - 2];
            relative_subpoint[1] += a[a.len() - 1];
        }
    }
    for s in &mut state.relative_subpoints[index] {
        *s = options.round(*s, state.error);
    }

    if let command::Data::MoveBy(_) | command::Data::MoveTo(_) = item.command {
        let relative_subpoint = state.relative_subpoints[index];
        state.base_path[0] = relative_subpoint[0];
        state.base_path[1] = relative_subpoint[1];
    };
}

fn update_relative_subpoint(
    item: &mut Position,
    options: &convert::Options,
    state: &mut filter::State,
    index: usize,
) {
    options.round_data(&mut state.relative_subpoints[index], state.error);
    match &item.command {
        command::Data::MoveBy(_)
        | command::Data::LineBy(_)
        | command::Data::SmoothQuadraticBezierBy(_)
        | command::Data::QuadraticBezierBy(_)
        | command::Data::SmoothBezierBy(_)
        | command::Data::CubicBezierBy(_) => {
            let relative_subpoint = state.relative_subpoints[index];
            let args = item.command.args_mut();
            for i in (0..args.len()).rev() {
                args[i] += item.start.0[i % 2] - relative_subpoint[i % 2];
            }
        }
        command::Data::HorizontalLineBy(mut a) => {
            let relative_subpoint = state.relative_subpoints[index];
            a[0] += item.start.0[0] - relative_subpoint[0];
        }
        command::Data::VerticalLineBy(mut a) => {
            let relative_subpoint = state.relative_subpoints[index];
            a[0] += item.start.0[1] - relative_subpoint[1];
        }
        command::Data::ArcBy(mut a) => {
            let relative_subpoint = state.relative_subpoints[index];
            a[5] += item.start.0[0] - relative_subpoint[0];
            a[6] += item.start.0[1] - relative_subpoint[1];
        }
        command::Data::Implicit(c) => {
            let mut new_position = Position {
                command: *c.clone(),
                start: item.start,
                end: item.end,
                s_data: item.s_data.clone(),
            };
            update_relative_subpoint(&mut new_position, options, state, index);
            new_position.command = command::Data::Implicit(Box::new(new_position.command));
            *item = new_position;
        }
        _ => {}
    }
}

pub fn arc_smart(item: &mut Position, options: &convert::Options, state: &mut filter::State) {
    if !options.flags.smart_arc_rounding() {
        return;
    }
    let Some(precision) = options.precision.inner() else {
        return;
    };
    if precision == 0 {
        return;
    }
    let command::Data::ArcBy(ref mut args) = item.command else {
        return;
    };
    let Some(saggita) = math::saggita(args, state.error) else {
        return;
    };

    for precision_new in (0..precision).rev() {
        let radius = math::to_fixed(args[0], precision_new);
        let Some(saggita_new) = math::saggita(
            &[radius, radius, args[2], args[3], args[4], args[5], args[6]],
            state.error,
        ) else {
            break;
        };
        if f64::abs(saggita - saggita_new) < state.error {
            args[0] = radius;
            args[1] = radius;
        } else {
            break;
        }
    }
    state.saggita = Some(saggita);
}
