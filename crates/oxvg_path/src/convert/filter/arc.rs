use crate::{
    command::{self, Position},
    convert::{self, filter},
    geometry::{Circle, Curve, MakeArcs, Point},
    Path,
};

pub struct Convert {
    circle: Circle,
    radius: f64,
    sweep: f64,
    angle: f64,
    suffix: String,
    relative_circle: Circle,
    relative_center: Point,
    relative_subpoint: Point,
    output: Vec<Position>,
    arc_curves: Vec<Position>,
    arc: Position,
    has_prev: usize,
}

impl Convert {
    fn init(
        item: &Position,
        options: &convert::Options,
        state: &filter::State,
        s_data: &Curve,
    ) -> Option<Self> {
        let make_arcs = match &options.make_arcs {
            Some(a)
                if matches!(
                    item.command.id(),
                    command::ID::CubicBezierBy | command::ID::SmoothBezierBy
                ) && s_data.is_convex() =>
            {
                a
            }
            _ => return None,
        };
        let circle = Circle::find(s_data, make_arcs, state.error)?;
        let radius = options.round(circle.radius, state.error);
        let sweep = f64::from(s_data.0[5] * s_data.0[0] - s_data.0[4] * s_data.0[5] > 0.0);
        let relative_center = Point([
            circle.center.0[0] - s_data.0[4],
            circle.center.0[1] - s_data.0[5],
        ]);
        let arc = Position {
            command: command::Data::ArcBy([
                radius,
                radius,
                0.0,
                0.0,
                sweep,
                s_data.0[4],
                s_data.0[5],
            ]),
            start: item.start,
            end: item.end,
            s_data: None,
        };

        Some(Convert {
            circle: circle.clone(),
            radius,
            sweep,
            angle: circle.arc_angle(s_data),
            suffix: String::new(),
            relative_circle: Circle {
                center: relative_center,
                radius: circle.radius,
            },
            relative_center,
            relative_subpoint: Point::default(),
            arc_curves: vec![item.clone()],
            output: vec![arc.clone()],
            arc,
            has_prev: 0,
        })
    }

    /// # Panics
    /// If internal assertions fail
    pub fn curve(
        prev: &mut Position,
        item: &mut Position,
        next_paths: &mut [Option<Position>],
        options: &convert::Options,
        state: &filter::State,
        s_data: &Curve,
    ) {
        let Some(ref make_arcs) = options.make_arcs else {
            return;
        };

        let Some(mut arc_state) = Self::init(item, options, state, s_data) else {
            // Not a curve
            return;
        };

        // NOTE: At this point, `prev` and `item` are `Some(_)`
        // We keep them as `&mut Option<_>` so they may be replaced with `None` later
        arc_state.get_s_data_info(prev, make_arcs, state.error);
        arc_state.check_next_curves_fit(item, next_paths, make_arcs, state.error);

        let Convert {
            ref output,
            ref arc_curves,
            ref suffix,
            ..
        } = arc_state;
        let output_path = Path(output.clone().into_iter().map(|p| p.command).collect());
        let arc_curves_path = Path(arc_curves.clone().into_iter().map(|p| p.command).collect());
        if String::from(output_path).len() + suffix.len() < String::from(arc_curves_path).len() {
            arc_state.use_output_arc(prev, item, next_paths, options, s_data, state.error);
        }
    }

    fn get_s_data_info(&mut self, prev: &Position, make_arcs: &MakeArcs, error: f64) {
        let prev_s_data = match prev {
            Position {
                command: command::Data::CubicBezierBy(p),
                ..
            } => Some(Curve(*p)),
            Position {
                s_data: Some(p), ..
            } => Some(p.clone()),
            _ => None,
        };
        match prev_s_data {
            Some(prev_s_data)
                if prev_s_data.is_convex()
                    && prev_s_data.is_arc_prev(&self.circle, make_arcs, error) =>
            {
                let Convert {
                    ref mut arc,
                    ref mut circle,
                    ref mut angle,
                    ref mut has_prev,
                    ..
                } = self;
                self.arc_curves.insert(0, prev.clone());
                arc.start = prev.start;
                arc.command.set_arg(5, arc.end.0[0] - arc.start.0[0]);
                arc.command.set_arg(6, arc.end.0[1] - arc.start.0[1]);
                let prev_angle = Circle {
                    center: Point([
                        prev_s_data.0[4] + circle.center.0[0],
                        prev_s_data.0[5] + circle.center.0[1],
                    ]),
                    radius: circle.radius,
                }
                .arc_angle(&prev_s_data);
                *angle += prev_angle;
                if *angle > std::f64::consts::PI {
                    arc.command.set_arg(3, 1.0);
                }
                *has_prev = 1;
            }
            _ => {}
        }
    }

    fn check_next_curves_fit(
        &mut self,
        item: &Position,
        next_paths: &mut [Option<Position>],
        make_arcs: &MakeArcs,
        error: f64,
    ) {
        let prev = item;
        for next in next_paths
            .iter_mut()
            .filter_map(|p| p.as_mut())
            .take_while(|p| {
                matches!(
                    p.command,
                    command::Data::CubicBezierBy(_) | command::Data::SmoothBezierBy(_)
                )
            })
        {
            let next_data = match next.command {
                command::Data::SmoothBezierBy(_) => {
                    let mut longhand = next.command.make_longhand(prev.command.args());
                    let args = longhand.clone();
                    let args = args.args();
                    for i in 2..args.len() {
                        longhand.set_arg(i, 0.0);
                    }
                    self.suffix = String::from(Path(vec![longhand.clone()]));
                    [args[0], args[1], args[2], args[3], args[4], args[5]]
                }
                command::Data::CubicBezierBy(a) => a,
                _ => {
                    unreachable!("earlier `take_while` should have yielded only bezier-by commands")
                }
            };
            let next_data = Curve(next_data);
            if !next_data.is_convex() || !next_data.is_arc(&self.relative_circle, make_arcs, error)
            {
                break;
            }
            let Convert {
                arc,
                arc_curves,
                angle,
                output,
                relative_center,
                ref relative_circle,
                ref radius,
                ref sweep,
                ..
            } = self;
            *angle += relative_circle.arc_angle(&next_data);
            if *angle - 2.0 * std::f64::consts::PI > 1e-3 {
                // more than 360deg
                break;
            };
            if *angle > std::f64::consts::PI {
                arc.command.set_arg(3, 1.0);
            }
            arc_curves.push(next.clone());
            if 2.0 * std::f64::consts::PI - *angle > 1e-3 {
                // less than 360deg
                arc.end = next.end;
                arc.command.set_arg(5, arc.end.0[0] - arc.start.0[0]);
                arc.command.set_arg(6, arc.end.0[1] - arc.start.0[1]);
                relative_center.0[0] -= next_data.0[4];
                relative_center.0[1] -= next_data.0[5];
            } else {
                // full circle, make a half-circle arc and add a second one
                let arc_args = arc.command.args_mut();
                arc_args[5] = 2.0 * (relative_circle.center.0[0] - next_data.0[4]);
                arc_args[6] = 2.0 * (relative_circle.center.0[1] - next_data.0[5]);
                arc.end = Point([arc.start.0[0] + arc_args[5], arc.start.0[1] + arc_args[6]]);
                *arc = Position {
                    command: command::Data::ArcBy([
                        *radius,
                        *radius,
                        0.0,
                        0.0,
                        *sweep,
                        next.end.0[0] - arc.end.0[0],
                        next.end.0[1] - arc.end.0[1],
                    ]),
                    start: arc.start,
                    end: next.end,
                    s_data: None,
                };
                output.push(arc.clone());
            }
        }
    }

    fn use_output_arc(
        &mut self,
        prev: &mut Position,
        item: &mut Position,
        next_paths: &mut [Option<Position>],
        options: &convert::Options,
        s_data: &Curve,
        error: f64,
    ) {
        if let command::Data::SmoothBezierBy(_) = item.command {
            item.command = item.command.make_longhand(prev.command.args());
        }
        let Convert {
            arc,
            output,
            relative_subpoint,
            arc_curves,
            ref has_prev,
            ..
        } = self;

        // Update prev command to arc
        let prev_args = prev.command.args();
        let mut prev_arc = output.remove(0);
        let prev_arc_args = prev_arc.command.args_mut();
        prev_arc_args
            .iter_mut()
            .for_each(|a| *a = options.round(*a, error));
        relative_subpoint.0[0] += prev_arc_args[5] - prev_args[prev_args.len() - 2];
        prev.command = command::Data::ArcBy(prev_arc_args.try_into().unwrap());
        prev.end = prev_arc.end;
        item.start = prev_arc.end;

        // Update item to arc
        let removable_arcs = arc_curves.len() - 1 - has_prev;
        if arc_curves.len() == 1 {
            item.s_data = Some(s_data.clone());
        } else if removable_arcs > 0 {
            next_paths
                .iter_mut()
                .take(removable_arcs)
                .for_each(|p| *p = None);
        }
        if output.is_empty() {
            return;
        }
        *arc = output.remove(0);
        item.command = command::Data::ArcBy(arc.command.args().try_into().unwrap());
        item.end = arc.end;
    }
}