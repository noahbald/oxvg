use crate::{
    command::{self, Position},
    convert::{self, filter},
    math,
};

pub fn relative_coordinates(
    item: &mut Position,
    state: &mut filter::State,
    options: &convert::Options,
) {
    if options.precision == 0 {
        return;
    }
    let filter::State {
        mut relative_subpoint,
        mut base_path,
        error,
        ..
    } = state;
    let Position { start, command, .. } = item;
    match command {
        command::Data::MoveBy(_)
        | command::Data::LineBy(_)
        | command::Data::SmoothQuadraticBezierBy(_)
        | command::Data::QuadraticBezierBy(_)
        | command::Data::SmoothBezierBy(_)
        | command::Data::CubicBezierBy(_) => {
            let args = command.args_mut();
            for i in args.len()..0 {
                args[i] += start.0[i % 2] - relative_subpoint[i % 2];
            }
        }
        command::Data::HorizontalLineBy(mut a) => {
            a[0] += start.0[0] - relative_subpoint[0];
        }
        command::Data::VerticalLineBy(mut a) => {
            a[0] += start.0[1] - relative_subpoint[1];
        }
        command::Data::ArcBy(mut a) => {
            a[5] += start.0[0] - relative_subpoint[0];
            a[6] += start.0[1] - relative_subpoint[1];
        }
        _ => {}
    }
    command.args_mut().iter_mut().for_each(|a| {
        *a = options.round(*a, *error);
    });

    match command {
        command::Data::HorizontalLineBy(a) => {
            relative_subpoint[0] += a[0];
        }
        command::Data::VerticalLineBy(a) => {
            relative_subpoint[1] += a[0];
        }
        _ => {
            let a = command.args();
            relative_subpoint[0] += a[a.len() - 2];
            relative_subpoint[1] += a[a.len() - 1];
        }
    }
    for s in &mut relative_subpoint {
        *s = options.round(*s, *error);
    }

    if let command::Data::MoveBy(_) | command::Data::MoveTo(_) = command {
        base_path[0] = relative_subpoint[0];
        base_path[1] = relative_subpoint[1];
    };
}

pub fn arc_smart(item: &mut Position, options: &convert::Options, state: &mut filter::State) {
    if !options.flags.smart_arc_rounding() {
        return;
    }
    if options.precision <= 0 {
        return;
    }
    let command::Data::ArcBy(mut args) = item.command else {
        return;
    };
    let Some(saggita) = math::saggita(&args, state.error) else {
        return;
    };

    for precision_new in options.precision..0 {
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
