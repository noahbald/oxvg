pub mod arc;
mod from;
mod remove;
mod round;

use crate::{
    command::{self, Position},
    convert,
    geometry::{Curve, Point},
    positioned::Path,
};

use super::StyleInfo;

#[derive(Clone)]
pub struct State<'a> {
    options: &'a convert::Options,
    info: &'a StyleInfo,
    pub relative_subpoints: Vec<[f64; 2]>,
    base_path: [f64; 2],
    prev_q_control_point: Option<Point>,
    saggita: Option<f64>,
    pub error: f64,
}

impl<'a> State<'a> {
    pub fn new(path: &Path, options: &'a convert::Options, info: &'a StyleInfo) -> Self {
        let mut state = Self {
            options,
            info,
            relative_subpoints: vec![[0.0; 2]; path.0.len()],
            base_path: [0.0; 2],
            prev_q_control_point: None,
            saggita: None,
            error: options.error(),
        };
        if let Some(item) = path.0.first() {
            state.base_path = item.end.0;
            state.relative_subpoints[0] = item.end.0;
        }
        state
    }
}

/// Filters unnecessary commands from a path with known positions, transforming the path if
/// necessary
///
/// # Panics
/// If the path length changes while running
pub fn filter(
    path: &Path,
    options: &convert::Options,
    state: &mut State,
    info: &StyleInfo,
) -> Path {
    let mut new_path: Vec<_> = path.0.clone().into_iter().map(Some).collect();
    (0..path.0.len()).for_each(|index| {
        if index > 0 {
            state.relative_subpoints[index] = state.relative_subpoints[index - 1];
        }
        let Some((prev, item_option, next_paths)) = Path::split_mut(&mut new_path, index) else {
            return;
        };
        let item = item_option
            .as_mut()
            .expect("`split_mut` guard would return if item is `None`");

        if remove::repeated_close_path(prev, item, state, index) {
            *item_option = None;
            return;
        }

        let s_data = Curve::smooth_bezier_by_args(prev, item);
        if let Some(ref s_data) = s_data {
            assert!(matches!(
                item.command,
                command::Data::SmoothBezierBy(_) | command::Data::CubicBezierBy(_)
            ));
            let arc_state = arc::Convert::curve(prev, item, next_paths, options, state, s_data);
            if arc_state.is_some_and(|s| s.remove_item) {
                *item_option = None;
                return;
            }
        }

        let next = match next_paths.split_first_mut() {
            Some((next, _)) => next,
            None => &mut None,
        };
        round::relative_coordinates(item, state, options, index);
        round::arc_smart(item, options, state);
        from::straight_curve_to_line(prev, item, next, &s_data, options, state);
        from::c_to_q(item, next, options, state.error);
        from::line_to_shorthand(item, options);
        if remove::repeated(prev, item, options, info) {
            *item_option = None;
            return;
        }
        from::curve_to_shorthand(prev, item, options, state);
        if remove::useless_segment(item, options, info) {
            *item_option = None;
            return;
        }
        from::home_to_z(item, next, options, state, info);

        state.prev_q_control_point = get_q_control_point(item, state.prev_q_control_point);
    });
    let result = Path(new_path.into_iter().flatten().collect());
    #[cfg(debug_assertions)]
    {
        let path_dbg = path.clone().take().to_string();
        let result_dbg = result.clone().take().to_string();
        if path_dbg != result_dbg {
            log::debug!("convert::filter: updated path {result_dbg}");
        }
    }
    result
}

fn get_q_control_point(item: &Position, q_control_point: Option<Point>) -> Option<Point> {
    match item.command {
        command::Data::QuadraticBezierBy(a) => {
            Some(Point([a[0] + item.start.0[0], a[1] + item.start.0[1]]))
        }
        command::Data::SmoothQuadraticBezierBy(_) => {
            if let Some(q_control_point) = q_control_point {
                Some(q_control_point.reflect(item.start))
            } else {
                Some(item.end)
            }
        }
        _ => None,
    }
}
