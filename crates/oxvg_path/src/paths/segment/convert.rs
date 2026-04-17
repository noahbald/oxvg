use std::f64::consts::PI;

use crate::{
    command,
    geometry::{Arc, Curve, Point, Polygon},
    paths::segment::{Data, Path, Segment, Tolerance},
};

impl Data {
    pub(crate) fn take<'a>(
        command: &command::Data,
        start: &mut Point,
        cursor: &mut Point,
        control: &mut Option<Point>,
        z_end: &mut bool,
    ) -> Option<Self> {
        let last_control = *control;
        *control = None;
        match command {
            command::Data::MoveBy(a) => {
                *cursor = *cursor + Point(*a);
                None
            }
            command::Data::MoveTo(a) => {
                *cursor = Point(*a);
                None
            }
            command::Data::LineBy(a) => {
                *cursor = *cursor + Point(*a);
                Some(Data::LineTo(*cursor))
            }
            command::Data::LineTo(a) => {
                *cursor = Point(*a);
                Some(Data::LineTo(*cursor))
            }
            command::Data::HorizontalLineBy(a) => {
                cursor.0[0] += a[0];
                Some(Data::LineTo(*cursor))
            }
            command::Data::HorizontalLineTo(a) => {
                cursor.0[0] = a[0];
                Some(Data::LineTo(*cursor))
            }
            command::Data::VerticalLineBy(a) => {
                cursor.0[1] += a[0];
                Some(Data::LineTo(*cursor))
            }
            command::Data::VerticalLineTo(a) => {
                cursor.0[1] = a[0];
                Some(Data::LineTo(*cursor))
            }
            command::Data::CubicBezierBy(a) => {
                let start_control = *cursor + Point([a[0], a[1]]);
                let end_control = *cursor + Point([a[2], a[3]]);
                let end_point = *cursor + Point([a[4], a[5]]);

                *control = Some(end_control);
                *cursor = end_point;
                Some(Self::CurveTo(Curve::new(
                    start_control,
                    end_control,
                    end_point,
                )))
            }
            command::Data::CubicBezierTo(a) => {
                let end_control = Point([a[2], a[3]]);
                let end_point = Point([a[4], a[5]]);

                *control = Some(end_control);
                *cursor = end_point;
                Some(Data::CurveTo(Curve(*a)))
            }
            command::Data::QuadraticBezierBy(a) => {
                let quad_control = *cursor + Point([a[0], a[1]]);
                let end = *cursor + Point([a[2], a[3]]);
                *control = Some(quad_control);

                let (cp1, cp2) = Point::quadratic_control_points(quad_control, *cursor, end);
                *cursor = end;
                Some(Data::CurveTo(Curve::new(cp1, cp2, *cursor)))
            }
            command::Data::QuadraticBezierTo(a) => {
                let quad_control = Point([a[0], a[1]]);
                let end = Point([a[2], a[3]]);
                *control = Some(quad_control);

                let (cp1, cp2) = Point::quadratic_control_points(quad_control, *cursor, end);
                *cursor = end;
                Some(Data::CurveTo(Curve::new(cp1, cp2, end)))
            }
            command::Data::SmoothBezierBy(a) => {
                let start_control = if let Some(prev_cp) = last_control {
                    prev_cp.reflect(*cursor)
                } else {
                    *cursor
                };
                let end_control = *cursor + Point([a[0], a[1]]);
                let end = *cursor + Point([a[2], a[3]]);

                *control = Some(end_control);
                *cursor = end;
                Some(Data::CurveTo(Curve::new(start_control, end_control, end)))
            }
            command::Data::SmoothBezierTo(a) => {
                let start_control = if let Some(prev_cp) = last_control {
                    prev_cp.reflect(*cursor)
                } else {
                    *cursor
                };
                let end_control = Point([a[0], a[1]]);
                let end = Point([a[2], a[3]]);

                *control = Some(end_control);
                *cursor = end;
                Some(Data::CurveTo(Curve::new(start_control, end_control, end)))
            }
            command::Data::SmoothQuadraticBezierBy(a) => {
                let start_control = if let Some(prev_cp) = last_control {
                    prev_cp.reflect(*cursor)
                } else {
                    *cursor
                };
                let end = *cursor + Point(*a);

                *control = Some(start_control);
                let (cp1, cp2) = Point::quadratic_control_points(start_control, *cursor, end);
                *cursor = end;
                Some(Data::CurveTo(Curve::new(cp1, cp2, end)))
            }
            command::Data::SmoothQuadraticBezierTo(a) => {
                let start_control = if let Some(prev_cp) = last_control {
                    prev_cp.reflect(*cursor)
                } else {
                    *cursor
                };
                let end = Point(*a);

                *control = Some(start_control);
                let (cp1, cp2) = Point::quadratic_control_points(start_control, *cursor, end);
                *cursor = end;
                Some(Data::CurveTo(Curve::new(cp1, cp2, end)))
            }
            command::Data::ArcBy(a) => Some(
                Arc::from_arc_by(*a, cursor)
                    .map(Self::ArcTo)
                    .unwrap_or_else(|| {
                        *cursor = *cursor + Point([a[5], a[6]]);
                        Self::LineTo(*cursor)
                    }),
            ),
            command::Data::ArcTo(a) => Some(
                Arc::from_arc_to(*a, cursor)
                    .map(Self::ArcTo)
                    .unwrap_or_else(|| {
                        *cursor = Point([a[5], a[6]]);
                        Self::LineTo(*cursor)
                    }),
            ),
            command::Data::ClosePath => {
                *z_end = true;
                None
            }
            command::Data::Implicit(command) => Self::take(command, start, cursor, control, z_end),
        }
    }
}

impl Segment {
    fn take<'a>(
        iterator: &mut impl Iterator<Item = &'a command::Data>,
        start: &mut Point,
        cursor: &mut Point,
        tolerance: &Tolerance,
    ) -> Option<Self> {
        let mut result = Segment {
            start: *start,
            data: vec![],
            closed: false,
        };
        let mut z_end = false;
        let mut control = None;
        while let Some(command) = iterator.next() {
            if let Some(data) = Data::take(command, start, cursor, &mut control, &mut z_end) {
                result.data.push(data);
            } else {
                break;
            }
        }
        if result.data.is_empty() {
            return None;
        }
        if z_end {
            result.data.push(Data::LineTo(result.start));
            *cursor = result.start;
            result.closed = true;
            return Some(result);
        }

        let tolerance_squared = tolerance.positional * tolerance.positional;
        result.closed =
            start.distance_squared(&result.data().last().unwrap().end_point()) < tolerance_squared;
        Some(result)
    }

    /// Coverts the segment to a polygon, dividing curves and arcs up to some tolerance.
    pub fn to_polygon(&self, tolerance: &Tolerance) -> Polygon {
        let mut points = vec![*self.start()];
        for item in self.data() {
            match item {
                Data::LineTo(point) => points.push(*point),
                Data::CurveTo(curve) => {
                    let start = points.last().copied().unwrap();
                    Polygon::from_curve(&mut points, start, *curve, &tolerance.square())
                }
                Data::ArcTo(arc) => {
                    let start = points.last().copied().unwrap();
                    Polygon::from_arc(&mut points, start, *arc, &tolerance.square())
                }
            }
        }
        Polygon {
            points,
            closed: self.closed(),
        }
    }
}

impl Path {
    pub fn from_svg(value: &crate::Path, tolerance: &Tolerance) -> Self {
        let start = &mut Point::default();
        let cursor = &mut Point::default();

        let mut result = Path(vec![]);
        let mut iterator = value.0.iter().peekable();
        while iterator.peek().is_some() {
            if let Some(component) = Segment::take(&mut iterator, start, cursor, tolerance) {
                result.0.push(component);
            }
        }
        result
    }
}

impl Path {
    pub fn to_svg(&self, tolerance: &Tolerance) -> crate::Path {
        let mut commands = vec![];
        let tolerance_squared = tolerance.positional * tolerance.positional;

        let mut end = Point::default();
        for segment in &self.0 {
            // Pick between `M` or `m` depending on serialized length
            let m_by = command::Data::MoveBy((segment.start - end).0);
            let m_to = command::Data::MoveTo(segment.start.0);
            commands.push(compactest(m_by, m_to));
            end = segment.start;

            let mut last_control: Option<Point> = None;
            for command in &segment.data {
                let control = last_control.take();
                let start = end;
                end = command.end_point();
                let by = end - start;
                let command = match command {
                    Data::LineTo(to) => {
                        if to.x() == start.x() {
                            let v_to = command::Data::VerticalLineTo([to.y()]);
                            let v_by = command::Data::VerticalLineBy([by.y()]);
                            compactest(v_by, v_to)
                        } else if to.y() == start.y() {
                            let h_to = command::Data::HorizontalLineTo([to.x()]);
                            let h_by = command::Data::HorizontalLineBy([by.x()]);
                            compactest(h_to, h_by)
                        } else if *to == segment.start {
                            command::Data::ClosePath
                        } else {
                            let l_to = command::Data::LineTo(to.0);
                            let l_by = command::Data::LineBy(by.0);
                            compactest(l_to, l_by)
                        }
                    }
                    Data::CurveTo(curve) => {
                        let start_control = curve.start_control();
                        let end_control = curve.end_control();
                        last_control = Some(end_control);
                        let is_quadratic =
                            start_control.distance_squared(&end_control) < tolerance_squared;

                        if is_quadratic {
                            let is_smooth = control.is_some_and(|c| {
                                c.reflect(end).distance_squared(&start_control) < tolerance_squared
                            });

                            if is_smooth {
                                let t_to = command::Data::SmoothQuadraticBezierTo(end.0);
                                let t_by = command::Data::SmoothQuadraticBezierBy(by.0);
                                compactest(t_to, t_by)
                            } else {
                                let start_control_by = start_control - start;
                                let q_to = command::Data::QuadraticBezierTo([
                                    start_control.x(),
                                    start_control.y(),
                                    end.x(),
                                    end.y(),
                                ]);
                                let q_by = command::Data::QuadraticBezierBy([
                                    start_control_by.x(),
                                    start_control_by.y(),
                                    end.x(),
                                    end.y(),
                                ]);
                                compactest(q_to, q_by)
                            }
                        } else {
                            let end_control_by = end_control - start;
                            let is_smooth = control.is_some_and(|c| {
                                c.reflect(start).distance_squared(&start_control)
                                    < tolerance_squared
                            });

                            if is_smooth {
                                let s_to = command::Data::SmoothBezierTo([
                                    end_control.x(),
                                    end_control.y(),
                                    end.x(),
                                    end.y(),
                                ]);
                                let s_by = command::Data::SmoothBezierBy([
                                    end_control_by.x(),
                                    end_control_by.y(),
                                    by.x(),
                                    by.y(),
                                ]);
                                compactest(s_to, s_by)
                            } else {
                                let start_control_by = start_control - start;
                                let c_to = command::Data::CubicBezierTo([
                                    start_control.x(),
                                    start_control.y(),
                                    end_control.x(),
                                    end_control.y(),
                                    end.x(),
                                    end.y(),
                                ]);
                                let c_by = command::Data::CubicBezierBy([
                                    start_control_by.x(),
                                    start_control_by.y(),
                                    end_control_by.x(),
                                    end_control_by.y(),
                                    by.x(),
                                    by.y(),
                                ]);
                                compactest(c_to, c_by)
                            }
                        }
                    }
                    Data::ArcTo(arc) => {
                        let a_to = [
                            arc.radii().x(),
                            arc.radii().y(),
                            arc.x_rotation(),
                            if arc.sweep_angle().abs() > PI {
                                1.0
                            } else {
                                0.0
                            },
                            if arc.sweep_angle() > 0.0 { 1.0 } else { 0.0 },
                            end.x(),
                            end.y(),
                        ];
                        let mut a_by = a_to;
                        a_by[5] = by.x();
                        a_by[6] = by.y();
                        let a_by = command::Data::ArcBy(a_by);
                        let a_to = command::Data::ArcTo(a_to);
                        compactest(a_by, a_to)
                    }
                };
                commands.push(command);
            }
        }
        crate::Path(commands)
    }
}

fn compactest(left: command::Data, right: command::Data) -> command::Data {
    if left.to_string().len() < right.to_string().len() {
        left
    } else {
        right
    }
}
