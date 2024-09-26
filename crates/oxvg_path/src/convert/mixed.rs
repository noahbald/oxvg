use crate::{
    command::{self, Position},
    convert, PositionedPath,
};

/// Writes data in the shortest form using absolute or relative coordinates.
///
/// # Panics
/// If internal assertions fail
pub fn mixed(path: &PositionedPath, options: &convert::Options) -> PositionedPath {
    let mut new_path: Vec<_> = path.0.clone().into_iter().map(Some).collect();
    (0..new_path.len()).for_each(|index| {
        let Some((prev, item_option, _)) = PositionedPath::split_mut(&mut new_path, index)
        else {
            return;
        };
        let item = item_option
            .as_mut()
            .expect("`split_mut` guard would have returned if item is `None`");

        if matches!(item.command, command::Data::ClosePath) {
            return;
        }
        if prev.command.id().next_implicit() == item.command.id() && index > 1 {
            item.command = command::Data::Implicit(Box::new(item.command.clone()));
        }

        let error = options.error();
        let mut absolute_command = to_absolute(item);
        if prev.command.id().next_implicit() == absolute_command.id() && index > 1 {
            absolute_command = command::Data::Implicit(Box::new(absolute_command));
        }
        options.round_data(absolute_command.args_mut(), error);
        let mut relative_command = item.command.clone();
        options.round_data(relative_command.args_mut(), error);

        let absolute_command_str = format!("{absolute_command}");
        let relative_command_str = format!("{relative_command}");
        if absolute_command_str.len() >= relative_command_str.len()
            && !options.flags.force_absolute_path()
        {
            return;
        }

        let args = item.command.args();
        let is_relative_better =
            options.flags.negative_extra_space()
            // will have space (l0 20l10 20 -> l0 20 10 20); and
            && item.command.id().is_implicit()
            // will omiting space be worth it; and
            && absolute_command_str.len() == relative_command_str.len() - 1
            // will omit space
            && (
                // omission via sign: l0 20 -10 20 -> l0 20-10 20
                args[0] < 0.0
                // omission via decimal: 10 20.1 .1 20 -> 10 20.1.1 20
                || (f64::floor(args[0]) == 0.0 && args[0].fract() > f64::EPSILON && prev.command.args().last().is_some_and(|a| a.fract() > f64::EPSILON)));

        if !is_relative_better || options.flags.force_absolute_path() {
        item.command = absolute_command;
        }
        if index == 1 && matches!(item.command, command::Data::LineBy(_) | command::Data::LineTo(_)) {
            item.command = command::Data::Implicit(Box::new(item.command.clone()));
        }
    });
    let result = PositionedPath(new_path.into_iter().flatten().collect());
    #[cfg(debug_assertions)]
    {
        let path_dbg = path.clone().take().to_string();
        let result_dbg = result.clone().take().to_string();
        if path_dbg != result_dbg {
            dbg!("convert::mixed: updated path", result_dbg);
        }
    }
    result
}

fn to_absolute(item: &Position) -> command::Data {
    match item.command {
        command::Data::MoveBy(_)
        | command::Data::LineBy(_)
        | command::Data::SmoothQuadraticBezierBy(_)
        | command::Data::QuadraticBezierBy(_)
        | command::Data::SmoothBezierBy(_)
        | command::Data::CubicBezierBy(_) => {
            let a: Vec<_> = item
                .command
                .args()
                .iter()
                .enumerate()
                .map(|(i, a)| a + item.start.0[i % 2])
                .collect();
            match item.command {
                command::Data::MoveBy(_) => command::Data::MoveTo(a.try_into().unwrap()),
                command::Data::LineBy(_) => command::Data::LineTo(a.try_into().unwrap()),
                command::Data::SmoothQuadraticBezierBy(_) => {
                    command::Data::SmoothQuadraticBezierTo(a.try_into().unwrap())
                }
                command::Data::QuadraticBezierBy(_) => {
                    command::Data::QuadraticBezierTo(a.try_into().unwrap())
                }
                command::Data::SmoothBezierBy(_) => {
                    command::Data::SmoothBezierTo(a.try_into().unwrap())
                }
                command::Data::CubicBezierBy(_) => {
                    command::Data::CubicBezierTo(a.try_into().unwrap())
                }
                _ => unreachable!(),
            }
        }
        command::Data::HorizontalLineBy(a) => {
            command::Data::HorizontalLineTo([a[0] + item.start.0[0]])
        }
        command::Data::VerticalLineBy(a) => command::Data::VerticalLineTo([a[0] + item.start.0[1]]),
        command::Data::ArcBy(a) => {
            let mut a = a.to_owned();
            a[5] += item.start.0[0];
            a[6] += item.start.0[1];
            command::Data::ArcTo(a)
        }
        _ => item.command.clone(),
    }
}