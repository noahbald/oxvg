use crate::{command, geometry::Point, positioned, Path};

/// Convert absolute path data coordinates to relative
pub fn relative(path: Path) -> positioned::Path {
    #[cfg(debug_assertions)]
    let original_dbg = path.to_string();

    let path = path;
    let start = &mut Point([0.0; 2]);
    let cursor = &mut Point([0.0; 2]);

    let result = positioned::Path(
        path.0
            .into_iter()
            .enumerate()
            .map(|(i, item)| convert_command_to_relative(item, start, cursor, i == 0))
            .collect(),
    );
    #[cfg(debug_assertions)]
    {
        let result_dbg = result.clone().take().to_string();
        if original_dbg != result_dbg {
            log::debug!("convert::relative: {original_dbg} changed to {result_dbg}",);
        }
    }
    result
}

#[allow(clippy::too_many_lines)]
fn convert_command_to_relative(
    mut command: command::Data,
    start: &mut Point,
    cursor: &mut Point,
    is_first: bool,
) -> command::Position {
    let base = *cursor;
    match command {
        command::Data::MoveBy(a) => {
            // Update start and cursor
            cursor.0[0] += a[0];
            cursor.0[1] += a[1];
            start.0[0] = cursor.0[0];
            start.0[1] = cursor.0[1];
        }
        command::Data::MoveTo(a) => {
            // M -> m
            let a = [a[0] - cursor.0[0], a[1] - cursor.0[1]];
            if is_first {
                // skip first moveto
                command = command::Data::MoveTo(a);
            } else {
                command = command::Data::MoveBy(a);
            }
            // update start and cursor
            cursor.0[0] += a[0];
            cursor.0[1] += a[1];
            start.0[0] = cursor.0[0];
            start.0[1] = cursor.0[1];
        }
        command::Data::LineBy(a) | command::Data::SmoothQuadraticBezierBy(a) => {
            cursor.0[0] += a[0];
            cursor.0[1] += a[1];
        }
        command::Data::LineTo(a) => {
            let a = [a[0] - cursor.0[0], a[1] - cursor.0[1]];
            command = command::Data::LineBy(a);
            cursor.0[0] += a[0];
            cursor.0[1] += a[1];
        }
        command::Data::HorizontalLineBy(a) => {
            cursor.0[0] += a[0];
        }
        command::Data::HorizontalLineTo(a) => {
            // H -> a
            let a = [a[0] - cursor.0[0]];
            command = command::Data::HorizontalLineBy(a);
            cursor.0[0] += a[0];
        }
        command::Data::VerticalLineBy(a) => {
            cursor.0[1] += a[0];
        }
        command::Data::VerticalLineTo(a) => {
            // V -> v
            let a = [a[0] - cursor.0[1]];
            command = command::Data::VerticalLineBy(a);
            cursor.0[1] += a[0];
        }
        command::Data::CubicBezierBy(a) => {
            cursor.0[0] += a[4];
            cursor.0[1] += a[5];
        }
        command::Data::CubicBezierTo(a) => {
            // C -> c
            let a = [
                a[0] - cursor.0[0],
                a[1] - cursor.0[1],
                a[2] - cursor.0[0],
                a[3] - cursor.0[1],
                a[4] - cursor.0[0],
                a[5] - cursor.0[1],
            ];
            command = command::Data::CubicBezierBy(a);
            cursor.0[0] += a[4];
            cursor.0[1] += a[5];
        }
        command::Data::SmoothBezierBy(a) | command::Data::QuadraticBezierBy(a) => {
            cursor.0[0] += a[2];
            cursor.0[1] += a[3];
        }
        command::Data::SmoothBezierTo(a) => {
            // S -> s
            let a = [
                a[0] - cursor.0[0],
                a[1] - cursor.0[1],
                a[2] - cursor.0[0],
                a[3] - cursor.0[1],
            ];
            command = command::Data::SmoothBezierBy(a);
            cursor.0[0] += a[2];
            cursor.0[1] += a[3];
        }
        command::Data::QuadraticBezierTo(a) => {
            let a = [
                a[0] - cursor.0[0],
                a[1] - cursor.0[1],
                a[2] - cursor.0[0],
                a[3] - cursor.0[1],
            ];
            command = command::Data::QuadraticBezierBy(a);
            cursor.0[0] += a[2];
            cursor.0[1] += a[3];
        }
        command::Data::SmoothQuadraticBezierTo(a) => {
            let a = [a[0] - cursor.0[0], a[1] - cursor.0[1]];
            command = command::Data::SmoothQuadraticBezierBy(a);
            cursor.0[0] += a[0];
            cursor.0[1] += a[1];
        }
        command::Data::ArcBy(a) => {
            cursor.0[0] += a[5];
            cursor.0[1] += a[6];
        }
        command::Data::ArcTo(mut a) => {
            a[5] -= cursor.0[0];
            a[6] -= cursor.0[1];
            command = command::Data::ArcBy(a);
            cursor.0[0] += a[5];
            cursor.0[1] += a[6];
        }
        command::Data::ClosePath => {
            cursor.0[0] = start.0[0];
            cursor.0[1] = start.0[1];
        }
        command::Data::Implicit(command) => {
            return convert_command_to_relative(*command, start, cursor, is_first);
        }
    }
    command::Position {
        command,
        start: base,
        end: *cursor,
        s_data: None,
    }
}

#[test]
fn test_convert_relative() {
    use crate::Path;
    use oxvg_parse::Parse as _;

    let mut path = Path::parse_string("M 10,50 C 20,30 40,50 60,70 C 10,20 30,40 50,60").unwrap();
    path = Path::from(relative(path));
    assert_eq!(
        String::from(path),
        String::from("M10 50c10-20 30 0 50 20c-50-50-30-30-10-10")
    );
}
