use crate::{
    command::{self, Position},
    convert,
    positioned::Path,
};

/// Writes data in the shortest form using absolute or relative coordinates.
///
/// # Panics
/// If internal assertions fail
pub fn mixed(path: &Path, options: &convert::Options) -> Path {
    let mut new_path: Vec<_> = path.0.clone().into_iter().map(Some).collect();
    (0..new_path.len()).for_each(|index| {
        let Some((prev, item_option, _)) = Path::split_mut(&mut new_path, index)
        else {
            return;
        };
        let item = item_option
            .as_mut()
            .expect("`split_mut` guard would have returned if item is `None`");

        if matches!(item.command, command::Data::ClosePath) {
            return;
        }

        let error = options.error();
        let mut absolute_command = to_absolute(item);
        options.round_absolute_command_data(absolute_command.args_mut(), error, &item.start.0);
        let mut relative_command = item.command.clone();
        options.round_data(relative_command.args_mut(), error);

        let absolute_command_str = format!("{absolute_command}");
        let relative_command_str = format!("{relative_command}");
        let absolute_command_max_len = absolute_command_str.len();
        let relative_command_max_len = relative_command_str.len();
        if absolute_command_max_len >= relative_command_max_len
            && !options.flags.force_absolute_path()
        {
            return;
        }

        let args = item.command.args();
        let is_relative_better =
            options.flags.negative_extra_space()
            // will have space (l0 20l10 20 -> l0 20 10 20); and
            // FIXME: BREAK: Should we use `prev.command.next_implicit()` instead?
            && prev.command.id() == item.command.id()
            // will omiting space be worth it; and
            && absolute_command_max_len == relative_command_max_len - 1
            // will omit space
            && (
                // omission via sign: l0 20 -10 20 -> l0 20-10 20
                args[0] < 0.0
                // omission via decimal: 10 20.1 .1 20 -> 10 20.1.1 20
                || (f64::floor(args[0]) == 0.0 && args[0].fract() > f64::EPSILON && prev.command.args().last().is_some_and(|a| a.fract() > f64::EPSILON)));

        if !is_relative_better || options.flags.force_absolute_path() {
            item.command = absolute_command;
        }
    });
    let result = Path(new_path.into_iter().flatten().collect());
    #[cfg(debug_assertions)]
    {
        let path_dbg = path.clone().take().to_string();
        let result_dbg = result.clone().take().to_string();
        if path_dbg != result_dbg {
            log::debug!("convert::mixed: updated path: {result_dbg}");
        }
    }
    result
}

/// Converts an absolute/relative command with positional information into an absolute command
pub fn to_absolute(item: &Position) -> command::Data {
    let s = &item.start.0;
    match item.command {
        command::Data::MoveBy(a) => command::Data::MoveTo([a[0] + s[0], a[1] + s[1]]),
        command::Data::LineBy(a) => command::Data::LineTo([a[0] + s[0], a[1] + s[1]]),
        command::Data::SmoothQuadraticBezierBy(a) => {
            command::Data::SmoothQuadraticBezierTo([a[0] + s[0], a[1] + s[1]])
        }
        command::Data::QuadraticBezierBy(a) => {
            command::Data::QuadraticBezierTo([a[0] + s[0], a[1] + s[1], a[2] + s[0], a[3] + s[1]])
        }
        command::Data::SmoothBezierBy(a) => {
            command::Data::SmoothBezierTo([a[0] + s[0], a[1] + s[1], a[2] + s[0], a[3] + s[1]])
        }
        command::Data::CubicBezierBy(a) => command::Data::CubicBezierTo([
            a[0] + s[0],
            a[1] + s[1],
            a[2] + s[0],
            a[3] + s[1],
            a[4] + s[0],
            a[5] + s[1],
        ]),
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
        command::Data::Implicit(ref c) => to_absolute(&Position {
            command: *c.clone(),
            start: item.start,
            end: item.end,
            s_data: item.s_data.clone(),
        }),
        _ => item.command.clone(),
    }
}
