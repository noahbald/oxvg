use crate::{
    command::{self, ID},
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
                    Polygon::from_curve(&mut points, start, curve, &tolerance.square())
                }
                Data::ArcTo(arc) => {
                    let start = points.last().copied().unwrap();
                    Polygon::from_arc(&mut points, start, arc, &tolerance.square())
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
    /// Returns an svg equivalent path from a segmented path.
    pub fn to_svg(&self, tolerance: &Tolerance) -> crate::Path {
        // NOTE: When comparing compactness, order of precendence is given by order of TO/BY commands.
        //       We should order it so that BY is preferenced for readability.
        let mut commands = vec![];
        let tolerance_squared = tolerance.square();
        let precision = tolerance.precision;

        let mut end = Point::default();
        for segment in &self.0 {
            // Pick between `M` or `m` depending on serialized length
            let m_by = command::Data::MoveBy((segment.start - end).0);
            let m_to = command::Data::MoveTo(segment.start.0);
            commands.push(compactest(m_by, m_to, precision));
            end = segment.start;

            let mut last_control: Option<Point> = None;
            for command in &segment.data {
                let control = last_control.take();
                let start = end;
                end = command.end_point();
                let by = end - start;

                let line_to = |to: &Point| {
                    if to.x() == start.x() {
                        let v_to = command::Data::VerticalLineTo([to.y()]);
                        let v_by = command::Data::VerticalLineBy([by.y()]);
                        compactest(v_by, v_to, precision)
                    } else if to.y() == start.y() {
                        let h_to = command::Data::HorizontalLineTo([to.x()]);
                        let h_by = command::Data::HorizontalLineBy([by.x()]);
                        compactest(h_by, h_to, precision)
                    } else if *to == segment.start {
                        command::Data::ClosePath
                    } else {
                        let l_to = command::Data::LineTo(to.0);
                        let l_by = command::Data::LineBy(by.0);
                        compactest(l_by, l_to, precision)
                    }
                };
                let arc_to = |arc: &Arc| {
                    if arc.is_straight(&tolerance) {
                        line_to(&arc.end_point())
                    } else {
                        let arc_to = arc.to_arc_to();
                        let mut a_by = arc_to;
                        a_by[5] = by.x();
                        a_by[6] = by.y();
                        let a_by = command::Data::ArcBy(a_by);
                        let a_to = command::Data::ArcTo(arc_to);
                        compactest(a_by, a_to, precision)
                    }
                };

                let command = match command {
                    Data::LineTo(to) => line_to(to),
                    Data::ArcTo(arc) => arc_to(arc),
                    Data::CurveTo(curve) => {
                        let start_control = curve.start_control();
                        let end_control = curve.end_control();
                        let to = curve.end_point();
                        last_control = Some(end_control);

                        if curve.is_straight(start, &tolerance_squared) {
                            line_to(&to)
                        } else {
                            let mut candidates = Vec::with_capacity(2);

                            if control.is_some_and(|cp| {
                                start_control.distance_squared(&cp.reflect(start))
                                    < *tolerance_squared
                            }) {
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

                            let quad_control = start + (start_control - start) * 1.5;
                            if quad_control.distance_squared(&(end + (end_control - end) * 1.5))
                                < *tolerance_squared
                            {
                                if control.is_some_and(|cp| {
                                    quad_control.distance_squared(&cp.reflect(start))
                                        < *tolerance_squared
                                }) {
                                    candidates.push(command::Data::SmoothQuadraticBezierBy(by.0));
                                    candidates.push(command::Data::SmoothQuadraticBezierTo(end.0));
                                } else {
                                    candidates.push(command::Data::QuadraticBezierBy([
                                        quad_control.x() - start.x(),
                                        quad_control.y() - start.y(),
                                        by.x(),
                                        by.y(),
                                    ]));
                                    candidates.push(command::Data::QuadraticBezierTo([
                                        quad_control.x(),
                                        quad_control.y(),
                                        end.x(),
                                        end.y(),
                                    ]));
                                }
                            }

                            if let Some(arc) =
                                Arc::fit_curve(curve, start, tolerance, &tolerance_squared)
                            {
                                candidates.push(arc_to(&arc));
                            }

                            let result = compactest_vec(candidates, precision);
                            match result.id() {
                                ID::QuadraticBezierTo
                                | ID::QuadraticBezierBy
                                | ID::SmoothQuadraticBezierTo
                                | ID::SmoothQuadraticBezierBy => last_control = Some(quad_control),

                                ID::ArcTo | ID::ArcBy => {}
                                _ => last_control = Some(end_control),
                            }
                            println!("");
                            result
                        }
                    }
                };
                commands.push(command);
            }
        }
        crate::Path(commands)
    }
}

fn compactest_vec(data: Vec<command::Data>, precision: i32) -> command::Data {
    data.into_iter()
        .map(|mut d| {
            d.round(precision);
            d
        })
        .map(|d| (d.to_string().len(), d))
        .min_by(|a, b| a.0.cmp(&b.0))
        .map(|d| d.1)
        .unwrap()
}

fn compactest(left: command::Data, right: command::Data, precision: i32) -> command::Data {
    compactest_vec(vec![left, right], precision)
}

#[cfg(test)]
mod test {
    use oxvg_parse::Parse as _;

    use crate::{
        geometry::Point,
        optimize::Tolerance,
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

        let path = path.to_svg(&tolerance);
        assert_eq!(path.to_string().as_str(), "m5 5v10h10V5H5");
    }

    #[test]
    fn cubic_curves() {
        let source = "m10 90c20 0 15-80 40-80s20 80 40 80";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);
        let path = path.to_svg(&tolerance);
        assert_eq!(path.to_string().as_str(), source);
    }

    #[test]
    fn quadratic_curves() {
        let source = "M10 50Q25 25 40 50t30 0 30 0 30 0 30 0 30 0";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);
        let path = path.to_svg(&tolerance);
        assert_eq!(
            path.to_string().as_str(),
            "m10 50q15-25 30 0t30 0t30 0t30 0t30 0t30 0"
        );
    }

    #[test]
    fn arc() {
        let source = "M6 10A6 4 10 1 0 14 10";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);
        let path = path.to_svg(&tolerance);
        assert_eq!(path.to_string().as_str(), "m6 10a6 4 10 1 0 8 0");
    }

    #[test]
    fn curve_to_arc() {
        let source = "M0 0L0 0c2.761 0 5 2.239 5 5";
        let path = crate::Path::parse_string(source).unwrap();
        let tolerance = Tolerance::default();
        let path = Path::from_svg(&path, &tolerance);
        let path = path.to_svg(&tolerance);
        assert_eq!(path.to_string().as_str(), "m0 0v0a5 5 0 1 1 5 5");
    }
}
