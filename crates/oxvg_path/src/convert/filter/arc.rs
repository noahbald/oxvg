//! Methods for filtering arc commands
use crate::{
    command::{self, Position},
    convert::{self, filter},
    geometry::{Circle, Curve, MakeArcs, Point},
    Path,
};
use std::f64::consts::PI;

#[derive(Debug)]
/// The state of arc conversion
pub struct Convert {
    circle: Circle,
    radius: f64,
    sweep: f64,
    angle: f64,
    suffix: String,
    relative_circle: Circle,
    pub(crate) relative_subpoint: Point,
    output: Vec<Position>,
    arc_curves: Vec<Position>,
    has_prev: usize,
    pub(crate) remove_item: bool,
}

impl Convert {
    fn init(
        item: &Position,
        options: &convert::Options,
        state: &filter::State,
        s_data: &Curve,
    ) -> Option<Self> {
        if !matches!(
            item.command.id(),
            command::ID::CubicBezierBy | command::ID::SmoothBezierBy
        ) || !s_data.is_convex()
        {
            return None;
        }
        let make_arcs = &options.make_arcs;
        let circle = Circle::find(s_data, make_arcs, state.error)?;
        let radius = options.round(circle.radius, state.error);
        let sweep = f64::from(s_data.0[5] * s_data.0[0] - s_data.0[4] * s_data.0[1] > 0.0);
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

        let angle = circle.arc_angle(s_data);
        let radius = circle.radius;
        Some(Convert {
            circle,
            radius,
            sweep,
            angle,
            suffix: String::new(),
            relative_circle: Circle {
                center: relative_center,
                radius,
            },
            relative_subpoint: Point::default(),
            arc_curves: vec![item.clone()],
            output: vec![arc],
            has_prev: 0,
            remove_item: false,
        })
    }

    /// Converts curves into arcs where possible, otherwise will convert into their best
    /// alternative
    pub fn curve(
        prev: &mut Position,
        item: &mut Position,
        next_paths: &mut [Option<Position>],
        options: &convert::Options,
        state: &filter::State,
        s_data: &Curve,
    ) -> Option<Self> {
        let make_arcs = &options.make_arcs;

        let Some(mut arc_state) = Self::init(item, options, state, s_data) else {
            // Not a curve
            return None;
        };

        // NOTE: At this point, `prev` and `item` are `Some(_)`
        // We keep them as `&mut Option<_>` so they may be replaced with `None` later
        arc_state.get_s_data_info(prev, make_arcs, state.error);
        arc_state.check_next_curves_fit(item, next_paths, make_arcs, options, state.error);

        let Convert {
            ref output,
            ref arc_curves,
            ref suffix,
            ..
        } = arc_state;
        let mut output_path = Path(output.clone().into_iter().map(|p| p.command).collect());
        // Round for string length comparison
        options.round_path(&mut output_path, state.error);
        let mut arc_curves_path = Path(arc_curves.clone().into_iter().map(|p| p.command).collect());
        options.round_path(&mut arc_curves_path, state.error);
        if String::from(output_path).len() + suffix.len() < String::from(arc_curves_path).len() {
            arc_state.use_output_arc(prev, item, next_paths, options, s_data, state.error);
        }
        Some(arc_state)
    }

    /// For a bezier curve, gets the data regarding it's smooth-bezier args equivalent
    fn get_s_data_info(&mut self, prev: &Position, make_arcs: &MakeArcs, error: f64) {
        let prev_s_data = match prev {
            Position {
                command: command::Data::CubicBezierBy(p),
                ..
            } => Curve(*p),
            Position {
                s_data: Some(p), ..
            } => p.clone(),
            _ => return,
        };
        if prev_s_data.is_convex() && prev_s_data.is_arc_prev(&self.circle, make_arcs, error) {
            let Convert {
                ref mut output,
                ref mut circle,
                ref mut angle,
                ref mut has_prev,
                ..
            } = self;
            let arc = output
                .first_mut()
                .expect("output is initialised with one arc");
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
            if *angle > PI {
                arc.command.set_arg(3, 1.0);
            }
            *has_prev = 1;
        }
    }

    /// Checks whether the next curves continue the current item
    fn check_next_curves_fit(
        &mut self,
        item: &Position,
        next_paths: &mut [Option<Position>],
        make_arcs: &MakeArcs,
        options: &convert::Options,
        error: f64,
    ) {
        let mut prev = item;
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
                command::Data::SmoothBezierBy(args) => {
                    let args = command::Data::make_s_args_longhand(args, prev.command.args());
                    // NOTE: Command type doesn't matter here, it's used to measure an arbitrary 2
                    // arg command
                    let mut suffix = Path(vec![command::Data::MoveTo([args[0], args[1]])]);
                    options.round_path(&mut suffix, error);
                    self.suffix = String::from(suffix);
                    args
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
                arc_curves,
                angle,
                output,
                relative_circle,
                ref radius,
                ref sweep,
                ..
            } = self;
            let mut arc = output
                .last()
                .expect("output is initialised with at least one arc, which is never removed")
                .clone();
            *angle += relative_circle.arc_angle(&next_data);
            if *angle - 2.0 * PI > 1e-3 {
                // more than 360deg
                break;
            }
            if *angle > PI {
                arc.command.set_arg(3, 1.0);
            }
            arc_curves.push(next.clone());
            if 2.0 * PI - *angle > 1e-3 {
                // less than 360deg
                arc.end = next.end;
                arc.command.set_arg(5, arc.end.0[0] - arc.start.0[0]);
                arc.command.set_arg(6, arc.end.0[1] - arc.start.0[1]);
                relative_circle.center.0[0] -= next_data.0[4];
                relative_circle.center.0[1] -= next_data.0[5];
                let old_arc = output
                    .last_mut()
                    .expect("output is initialised with an arc, which is never removed");
                *old_arc = arc;
            } else {
                // full circle, make a half-circle arc and add a second one
                let arc_args = arc.command.args_mut();
                arc_args[5] = 2.0 * (relative_circle.center.0[0] - next_data.0[4]);
                arc_args[6] = 2.0 * (relative_circle.center.0[1] - next_data.0[5]);
                arc.end = Point([arc.start.0[0] + arc_args[5], arc.start.0[1] + arc_args[6]]);
                let old_arc = output
                    .last_mut()
                    .expect("output is initialised with an arc, which is never removed");
                *old_arc = arc.clone();
                arc = Position {
                    command: command::Data::ArcBy([
                        *radius,
                        *radius,
                        0.0,
                        0.0,
                        *sweep,
                        next.end.0[0] - arc.end.0[0],
                        next.end.0[1] - arc.end.0[1],
                    ]),
                    start: arc.end,
                    end: next.end,
                    s_data: None,
                };
                output.push(arc);
            }
            prev = next;
        }
    }

    /// Replaces all commands fitting the curve with a single command
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
            item.command.make_longhand(prev.command.args());
        }
        let Convert {
            output,
            relative_subpoint,
            arc_curves,
            ref has_prev,
            ..
        } = self;

        // Update prev command to arc
        if has_prev > &0 {
            let prev_args = prev.command.args();
            let mut prev_arc = output.remove(0);
            let prev_arc_args = prev_arc.command.args_mut();
            prev_arc_args
                .iter_mut()
                .for_each(|a| *a = options.round(*a, error));
            relative_subpoint.0[0] += prev_arc_args[5] - prev_args[prev_args.len() - 2];
            relative_subpoint.0[1] += prev_arc_args[6] - prev_args[prev_args.len() - 1];
            prev.command = command::Data::ArcBy(prev_arc_args.try_into().unwrap());
            prev.end = prev_arc.end;
            item.start = prev_arc.end;
        }

        // Update item to arc
        let removable_arcs = arc_curves.len() - 1 - has_prev;
        if arc_curves.len() == 1 {
            item.s_data = Some(s_data.clone());
        } else if removable_arcs > 0 {
            next_paths
                .iter_mut()
                .take(removable_arcs)
                .enumerate()
                .for_each(|(i, p)| *p = output.get(i + 1).cloned());
        }
        if output.is_empty() {
            self.remove_item = true;
            return;
        }
        let arc = output.remove(0);
        item.command = arc.command.clone();
        item.end = arc.end;
    }

    /// Gets curves(???) from an absolute arc command and it's base point.
    ///
    /// Based on code from Snap.svg (Apache 2 license). <http://snapsvg.io>/
    /// Thanks to Dmitry Baranovskiy for his great work!
    #[allow(clippy::similar_names, clippy::too_many_lines)]
    pub fn a2c<'a>(
        base_point: &'a [f64; 2],
        data: &'a [f64; 7],
        recursive: Option<&[f64]>,
    ) -> Vec<f64> {
        let [mut x1, mut y1] = *base_point;
        let [mut rx, mut ry, angle, large_arc_flag, sweep_flag, mut x2, mut y2] = *data;
        // for more information of where this Math came from visit:
        // https://www.w3.org/TR/SVG11/implnote.html#ArcImplementationNotes
        let rad = (PI / 180.0) * angle;
        let mut res = vec![];

        let (f1, mut f2, cx, cy) = if let Some(recursive) = recursive {
            (recursive[0], recursive[1], recursive[2], recursive[3])
        } else {
            x1 = rotate_x(x1, y1, -rad);
            y1 = rotate_y(x1, y1, -rad);
            x2 = rotate_x(x2, y2, -rad);
            y2 = rotate_y(x2, y2, -rad);
            let x = (x1 - x2) / 2.0;
            let y = (y1 - y2) / 2.0;
            let mut h = (x * x) / (rx * rx) + (y * y) / (ry * ry);
            let (rx, ry) = if h > 1.0 {
                h = h.sqrt();
                rx *= h;
                ry *= h;
                (rx, ry)
            } else {
                (rx, ry)
            };
            let rx2 = rx * rx;
            let ry2 = ry * ry;
            let k = if large_arc_flag == (if sweep_flag == 0.0 { 1.0 } else { -1.0 }) {
                f64::sqrt(f64::abs(
                    (rx2 * ry2 - rx2 * y * y - ry2 * x * x) / (rx2 * y * y + ry2 * x * x),
                ))
            } else {
                0.0
            };
            let cx = (k * rx * y) / ry + (x1 + x2) / 2.0;
            let cy = (k * -ry * x) / rx + (y1 + y2) / 2.0;
            let f1 = f64::asin((y1 - cy) / ry);
            let f2 = f64::asin((y2 - cy) / ry);

            let mut f1 = if x1 < cx { PI - f1 } else { f1 };
            let mut f2 = if x2 < cx { PI - f2 } else { f2 };
            if f1 < 0.0 {
                f1 += PI * 2.0;
            }
            if f2 < 0.0 {
                f2 += PI * 2.0;
            }
            if sweep_flag != 0.0 && f1 > f2 {
                f1 -= PI * 2.0;
            }
            if sweep_flag == 0.0 && f2 > f1 {
                f2 -= PI * 2.0;
            }
            (f1, f2, cx, cy)
        };
        let df = f2 - f1;
        if f64::abs(df) > _120 {
            let f2_old = f2;
            let x2_old = x2;
            let y2_old = y2;
            f2 = f1
                + _120
                    * (if sweep_flag != 0.0 && f2 > f1 {
                        1.0
                    } else {
                        -1.0
                    });
            x2 = cx + rx * f64::cos(f2);
            y2 = cy + ry * f64::sin(f2);
            res = Convert::a2c(
                &[x2, y2],
                &[rx, ry, angle, 0.0, sweep_flag, x2_old, y2_old],
                Some(&[f2, f2_old, cx, cy]),
            );
        }
        let df = f2 - f1;
        let c1 = f1.cos();
        let s1 = f1.sin();
        let c2 = f2.cos();
        let s2 = f2.sin();
        let t = f64::tan(df / 4.0);
        let hx = (4.0 / 3.0) * rx * t;
        let hy = (4.0 / 3.0) * ry * t;
        let m = [
            -hx * s1,
            hy * c1,
            x2 + hx * s2 - x1,
            y2 - hy * c2 - y1,
            x2 - x1,
            y2 - y1,
        ];
        let res = m.into_iter().chain(res).collect();
        if recursive.is_some() {
            res
        } else {
            (0..res.len())
                .map(|i| {
                    if i % 2 == 1 {
                        rotate_y(res[i - 1], res[i], rad)
                    } else {
                        rotate_x(res[i], res[i + 1], rad)
                    }
                })
                .collect()
        }
    }
}

fn rotate_x(x: f64, y: f64, rad: f64) -> f64 {
    x * f64::cos(rad) - y * f64::sin(rad)
}

fn rotate_y(x: f64, y: f64, rad: f64) -> f64 {
    x * f64::sin(rad) + y * f64::cos(rad)
}

const _120: f64 = (PI * 120.0) / 180.0;

#[test]
#[allow(clippy::unreadable_literal)]
fn a2c() {
    pretty_assertions::assert_eq!(
        println!(
            "{:.12?}",
            Convert::a2c(&[10.0, 10.0], &[6.0, 4.0, 10.0, 1.0, 0.0, 24.0, 20.0], None)
        ),
        println!(
            "{:.12?}",
            vec![
                -4.7584157403801,
                3.2058197930361305,
                -1.8799473690996882,
                9.267547431273137,
                5.1812430683047435,
                10.911109748826613,
                3.2771099921148092,
                0.7627799506704301,
                6.656316411051708,
                0.3161330070672036,
                8.864704831995782,
                -1.1716932404506106,
            ]
        )
    );
}
