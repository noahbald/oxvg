use crate::{
    command::{self, ID},
    geometry::{Arc, Curve, Point, Polygon, QuadraticBezierTo, SmoothBezierTo},
    math::{self},
    paths::segment::{
        Data, IterStartCursorItem, Path, Segment, Tolerance, TolerancePrecision, ToleranceSquared,
    },
};

impl Data {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn take(
        command: &command::Data,
        start: &mut Point,
        cursor: &mut Point,
        control: &mut Option<Point>,
        z_end: &mut bool,
    ) -> Option<Self> {
        let last_control = control.take();
        match command {
            command::Data::MoveBy(a) => {
                *cursor = *cursor + Point(*a);
                *start = *cursor;
                None
            }
            command::Data::MoveTo(a) => {
                *cursor = Point(*a);
                *start = *cursor;
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
            command::Data::Implicit(command) => {
                *control = last_control;
                Self::take(command, start, cursor, control, z_end)
            }
        }
    }
}

impl Segment {
    fn take<'a>(
        iterator: &mut impl Iterator<Item = &'a command::Data>,
        start: &mut Point,
        cursor: &mut Point,
        tolerance: &Tolerance,
    ) -> Self {
        let mut result = Segment {
            start: *start,
            data: vec![],
            closed: false,
        };
        let mut z_end = false;
        let mut control = None;
        for command in iterator {
            if let Some(data) = Data::take(command, start, cursor, &mut control, &mut z_end) {
                result.data.push(data);
            } else {
                break;
            }
        }
        if result.data.is_empty() {
            return result;
        }
        if z_end {
            *cursor = result.start;
            result.closed = true;
            return result;
        }

        let tolerance_squared = tolerance.positional * tolerance.positional;
        result.closed =
            start.distance_squared(&result.data().last().unwrap().end_point()) < tolerance_squared;
        result
    }

    /// Coverts the segment to a polygon, dividing curves and arcs up to some tolerance.
    ///
    /// # Panics
    ///
    /// If the generated polygons have to points.
    pub fn to_polygon(&self, tolerance: &Tolerance) -> Polygon {
        let mut points = vec![*self.start()];
        for item in self.data() {
            match item {
                Data::LineTo(point) => points.push(*point),
                Data::CurveTo(curve) => {
                    let start = points.last().copied().unwrap();
                    Polygon::from_curve(&mut points, start, curve, &tolerance.square());
                }
                Data::ArcTo(arc) => {
                    let start = points.last().copied().unwrap();
                    Polygon::from_arc(&mut points, start, arc, &tolerance.square(), 0);
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
    /// Returns the segment path from a parsed svg path.
    pub fn from_svg(value: &crate::Path, tolerance: &Tolerance) -> Self {
        let mut result = Path(vec![]);
        let mut iterator = value.0.iter().peekable();
        let mut start = match iterator.next() {
            Some(command::Data::MoveTo(p) | command::Data::MoveBy(p)) => Point(*p),
            None => return result,
            _ => unreachable!("Path data starts with non-move command"),
        };
        let mut cursor = start;
        while iterator.peek().is_some() {
            let component = Segment::take(&mut iterator, &mut start, &mut cursor, tolerance);
            result.0.push(component);
        }
        if result.0.is_empty() {
            result.0.push(Segment {
                start,
                data: vec![],
                closed: false,
            });
        }
        result
    }
}

impl Path {
    /// Returns an svg equivalent path from a segmented path, in the compactest set of equivalent commands.
    pub fn to_svg(&self, tolerance: &Tolerance, smart_arc_rounding: bool) -> crate::Path {
        // NOTE: When comparing compactness, order of precendence is given by order of TO/BY commands.
        //       We should order it so that BY is preferenced for readability.
        let tolerance_squared = tolerance.square();
        let precision = tolerance.precision();

        let mut commands: Vec<command::Data> = vec![];

        let mut last_control = None;
        for IterStartCursorItem {
            segment_start,
            segment_start_by,
            cursor: start,
            data,
            next,
            command,
            close,
        } in self.iter_start_cursor()
        {
            let control = last_control.take();
            let Some(data) = data else {
                let mut m = command::Data::MoveTo(start.0);
                m.round(&precision);
                commands.push(m);
                continue;
            };
            let is_start = command == 0;
            let previous = if is_start { None } else { commands.last() };
            let implicit = previous.map(|d| d.id().next_implicit());

            let mut command = match data {
                Data::LineTo(to) => Self::to_svg_line(
                    previous,
                    segment_start,
                    start,
                    *to,
                    implicit.as_ref(),
                    &precision,
                ),
                Data::ArcTo(arc) => Self::to_svg_arc(
                    previous,
                    arc,
                    start,
                    implicit.as_ref(),
                    tolerance,
                    &tolerance_squared,
                    &precision,
                    smart_arc_rounding,
                ),
                Data::CurveTo(curve) => {
                    let (command, control) = Self::to_svg_curve(
                        previous,
                        control,
                        start,
                        curve,
                        next,
                        implicit.as_ref(),
                        &tolerance_squared,
                        &precision,
                    );
                    last_control = control;
                    command
                }
            };

            if is_start {
                let mut m = match command.id().as_explicit() {
                    ID::LineBy => {
                        command = command::Data::Implicit(Box::new(command));
                        command::Data::MoveBy(segment_start_by.0)
                    }
                    ID::LineTo => {
                        command = command::Data::Implicit(Box::new(command));
                        command::Data::MoveTo(segment_start.0)
                    }
                    _ => command::Data::MoveTo(segment_start.0),
                };
                m.round(&precision);
                commands.push(m);
            }
            let close = close && command.id() != ID::ClosePath;
            commands.push(command);
            if close {
                commands.push(command::Data::ClosePath);
            }
        }

        crate::Path(commands)
    }

    fn to_svg_line(
        previous: Option<&command::Data>,
        segment_start: Point,
        start: Point,
        to: Point,
        implicit: Option<&ID>,
        precision: &TolerancePrecision,
    ) -> command::Data {
        let by = to - start;
        if start == to {
            command::Data::ClosePath
        } else if precision.round(to.x()) == precision.round(start.x()) {
            let v_to = command::Data::VerticalLineTo([to.y()]);
            let v_by = command::Data::VerticalLineBy([by.y()]);
            compactest(previous, v_by, v_to, implicit, precision)
        } else if precision.round(to.y()) == precision.round(start.y()) {
            let h_to = command::Data::HorizontalLineTo([to.x()]);
            let h_by = command::Data::HorizontalLineBy([by.x()]);
            compactest(previous, h_by, h_to, implicit, precision)
        } else if to == segment_start {
            command::Data::ClosePath
        } else {
            let l_to = command::Data::LineTo(to.0);
            let l_by = command::Data::LineBy(by.0);
            compactest(previous, l_by, l_to, implicit, precision)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn to_svg_curve(
        previous: Option<&command::Data>,
        control: Option<Point>,
        start: Point,
        curve: &Curve,
        next: Option<&Data>,
        implicit: Option<&ID>,
        tolerance_squared: &ToleranceSquared,
        precision: &TolerancePrecision,
    ) -> (command::Data, Option<Point>) {
        let start_control = curve.start_control();
        let end_control = curve.end_control();
        let to = curve.end_point();
        let by = to - start;

        let mut candidates = Vec::with_capacity(4);

        let (_, smooth_bezier, quadratic_bezier, smooth_quadratic_bezier) =
            curve.types(start, control, tolerance_squared);
        if let Some(SmoothBezierTo {
            end_control,
            end_point: _,
        }) = smooth_bezier
        {
            candidates.push(command::Data::SmoothBezierBy([
                end_control.x() - start.x(),
                end_control.y() - start.y(),
                by.x(),
                by.y(),
            ]));
            candidates.push(command::Data::SmoothBezierTo([
                end_control.x(),
                end_control.y(),
                to.x(),
                to.y(),
            ]));
        } else {
            candidates.push(command::Data::CubicBezierBy([
                start_control.x() - start.x(),
                start_control.y() - start.y(),
                end_control.x() - start.x(),
                end_control.y() - start.y(),
                by.x(),
                by.y(),
            ]));
            candidates.push(command::Data::CubicBezierTo([
                start_control.x(),
                start_control.y(),
                end_control.x(),
                end_control.y(),
                to.x(),
                to.y(),
            ]));
        }

        if smooth_quadratic_bezier.is_some() || quadratic_bezier.is_some() {
            let next_is_smooth_quadratic_bezier =
                quadratic_bezier.as_ref().is_some_and(|q| match next {
                    Some(Data::CurveTo(next)) => {
                        next.quad_control(to, tolerance_squared).is_some_and(|c| {
                            next.smooth_quadratic_bezier_unchecked_quad(
                                to,
                                Some(q.quad_control),
                                c,
                                tolerance_squared,
                            )
                            .is_some()
                        })
                    }
                    _ => false,
                });
            if next_is_smooth_quadratic_bezier {
                candidates.clear();
            }
        }
        if smooth_quadratic_bezier.is_some() {
            candidates.push(command::Data::SmoothQuadraticBezierBy(by.0));
            candidates.push(command::Data::SmoothQuadraticBezierTo(to.0));
        } else if let Some(QuadraticBezierTo {
            quad_control,
            end_point: _,
        }) = quadratic_bezier
        {
            candidates.push(command::Data::QuadraticBezierBy([
                quad_control.x() - start.x(),
                quad_control.y() - start.y(),
                by.x(),
                by.y(),
            ]));
            candidates.push(command::Data::QuadraticBezierTo([
                quad_control.x(),
                quad_control.y(),
                to.x(),
                to.y(),
            ]));
        }

        let result = compactest_vec(previous, candidates, implicit, precision);
        let control = match result.id().as_explicit() {
            ID::QuadraticBezierTo | ID::QuadraticBezierBy => {
                quadratic_bezier.map(|q| q.quad_control)
            }
            ID::SmoothQuadraticBezierTo | ID::SmoothQuadraticBezierBy => {
                control.map(|c| c.reflect(start))
            }
            ID::CubicBezierTo | ID::CubicBezierBy | ID::SmoothBezierBy | ID::SmoothBezierTo => {
                Some(quadratic_bezier.map_or(end_control, |q| q.quad_control))
            }
            _ => None,
        };
        (result, control)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn to_svg_arc(
        previous: Option<&command::Data>,
        arc: &Arc,
        start: Point,
        implicit: Option<&ID>,
        tolerance: &Tolerance,
        tolerance_squared: &ToleranceSquared,
        precision: &TolerancePrecision,
        smart_arc_rounding: bool,
    ) -> command::Data {
        let by = arc.end_point() - start;
        let mut arc_to = arc.to_arc_to(tolerance, tolerance_squared, precision);
        let mut arc_by = arc_to;
        arc_by[5] = by.x();
        arc_by[6] = by.y();

        if smart_arc_rounding {
            if let Some(saggita) = math::saggita(&arc_by, tolerance.positional) {
                let mut p = precision.0;
                let mut new_arc = arc_by;
                while p >= 1.0 {
                    new_arc[0] = TolerancePrecision(p).round(arc_by[0]);
                    new_arc[1] = TolerancePrecision(p).round(arc_by[1]);
                    p /= 10.0;
                    let Some(saggita_new) = math::saggita(&new_arc, tolerance.positional) else {
                        break;
                    };
                    if (saggita - saggita_new).abs() < tolerance.positional {
                        arc_by = new_arc;
                    } else {
                        break;
                    }
                }
                arc_to[0] = arc_by[0];
                arc_to[1] = arc_by[1];
            }
        }

        let a_by = command::Data::ArcBy(arc_by);
        let a_to = command::Data::ArcTo(arc_to);
        compactest(previous, a_by, a_to, implicit, precision)
    }
}

fn compactest_vec(
    previous: Option<&command::Data>,
    data: Vec<command::Data>,
    implicit: Option<&ID>,
    precision: &TolerancePrecision,
) -> command::Data {
    data.into_iter()
        .map(|d| {
            if implicit.is_some_and(|i| *i == d.id()) {
                command::Data::Implicit(Box::new(d))
            } else {
                d
            }
        })
        .map(|mut d| {
            d.round(precision);
            d
        })
        .map(|d| (d.to_string(), d))
        .map(|d| {
            if previous
                .as_ref()
                .is_some_and(|p| d.1.is_space_needed(p) && !d.0.starts_with('-'))
            {
                (d.0.len() + 1, d.1)
            } else {
                (d.0.len(), d.1)
            }
        })
        .min_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| b.1.is_implicit().cmp(&a.1.is_implicit()))
        })
        .map(|d| d.1)
        .unwrap()
}

pub(crate) fn compactest(
    previous: Option<&command::Data>,
    left: command::Data,
    right: command::Data,
    implicit: Option<&ID>,
    precision: &TolerancePrecision,
) -> command::Data {
    compactest_vec(previous, vec![left, right], implicit, precision)
}

#[cfg(test)]
mod test {
    use oxvg_parse::Parse as _;

    use crate::{
        geometry::Point,
        optimize::{Options, Tolerance},
        paths::segment::{Data, Path, Segment},
    };

    #[test]
    fn lines() {
        let source = "M5,5 L5,15 L15,15 L15,5 L5,5";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);

        assert_eq!(
            path,
            Path(vec![Segment {
                start: Point::splat(5.0),
                data: vec![
                    Data::LineTo(Point([5.0, 15.0])),
                    Data::LineTo(Point([15.0, 15.0])),
                    Data::LineTo(Point([15.0, 5.0])),
                    Data::LineTo(Point([5.0, 5.0]))
                ],
                closed: true
            }])
        );

        let path = path.to_svg(&tolerance, true);
        assert_eq!(path.to_string().as_str(), "M5 5v10h10V5H5Z");
    }

    #[test]
    fn cubic_curves() {
        let source = "m10 90c20 0 15-80 40-80s20 80 40 80";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);
        let path = path.to_svg(&tolerance, true);
        assert_eq!(
            path.to_string().as_str(),
            "M10 90c20 0 15-80 40-80s20 80 40 80"
        );
    }

    #[test]
    fn quadratic_curves() {
        let source = "M10 50Q25 25 40 50t30 0 30 0 30 0 30 0 30 0";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);
        let path = path.to_svg(&tolerance, true);
        assert_eq!(
            path.to_string().as_str(),
            "M10 50q15-25 30 0t30 0 30 0 30 0 30 0 30 0"
        );
    }

    #[test]
    fn arc() {
        let source = "M6 10A6 4 10 1 0 14 10";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);
        let path = path.to_svg(&tolerance, true);
        assert_eq!(path.to_string().as_str(), "M6 10a6 4 10 1 0 8 0");
    }

    #[test]
    fn curve_to_arc() {
        let source = "M0 0L0 0c2.761 0 5 2.239 5 5";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let mut path = Path::from_svg(&path, &tolerance);
        path.simplify(Options::all(), &tolerance);
        let path = path.to_svg(&tolerance, true);
        assert_eq!(path.to_string().as_str(), "M0 0a5 5 0 0 1 5 5");
    }
}
