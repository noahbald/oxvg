use crate::{
    command,
    geometry::{Arc, Curve, Point, Polygon},
    paths::segment::{Data, Path, Segment, DEFAULT_TOLERANCE},
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
                Some(Self::QuadTo(Curve::new(
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
                Some(Data::QuadTo(Curve(*a)))
            }
            command::Data::QuadraticBezierBy(a) => {
                let quad_control = *cursor + Point([a[0], a[1]]);
                let end = *cursor + Point([a[2], a[3]]);
                *control = Some(quad_control);

                let (cp1, cp2) = Point::quadratic_control_points(quad_control, *cursor, end);
                *cursor = end;
                Some(Data::QuadTo(Curve::new(cp1, cp2, *cursor)))
            }
            command::Data::QuadraticBezierTo(a) => {
                let quad_control = Point([a[0], a[1]]);
                let end = Point([a[2], a[3]]);
                *control = Some(quad_control);

                let (cp1, cp2) = Point::quadratic_control_points(quad_control, *cursor, end);
                *cursor = end;
                Some(Data::QuadTo(Curve::new(cp1, cp2, end)))
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
                Some(Data::QuadTo(Curve::new(start_control, end_control, end)))
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
                Some(Data::QuadTo(Curve::new(start_control, end_control, end)))
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
                Some(Data::QuadTo(Curve::new(cp1, cp2, end)))
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
                Some(Data::QuadTo(Curve::new(cp1, cp2, end)))
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

        result.closed = *start == result.data().last().unwrap().end_point();
        Some(result)
    }

    /// Coverts the segment to a polygon, dividing curves and arcs up to some tolerance.
    pub fn to_polygon(&self, tolerance: f64) -> Polygon {
        let mut points = vec![*self.start()];
        for item in self.data() {
            match item {
                Data::LineTo(point) => points.push(*point),
                Data::QuadTo(curve) => {
                    let start = points.last().copied().unwrap();
                    Polygon::from_curve(&mut points, start, *curve, tolerance)
                }
                Data::ArcTo(arc) => {
                    todo!("{arc:?}")
                }
            }
        }
        Polygon {
            points,
            closed: self.closed(),
        }
    }
}

impl From<Segment> for Polygon {
    fn from(value: Segment) -> Self {
        value.to_polygon(DEFAULT_TOLERANCE)
    }
}

impl From<&crate::Path> for Path {
    fn from(value: &crate::Path) -> Self {
        let start = &mut Point::default();
        let cursor = &mut Point::default();

        let mut result = Path(vec![]);
        let mut iterator = value.0.iter().peekable();
        while iterator.peek().is_some() {
            if let Some(component) = Segment::take(&mut iterator, start, cursor) {
                result.0.push(component);
            }
        }
        result
    }
}

impl Into<crate::Path> for &Path {
    fn into(self) -> crate::Path {
        let mut commands = vec![];

        let mut end = Point::default();
        for segment in &self.0 {
            // Pick between `M` or `m` depending on serialized length
            let m_by = command::Data::MoveBy((segment.start - end).0);
            let m_to = command::Data::MoveTo(segment.start.0);
            if m_by.to_string().len() < m_to.to_string().len() {
                commands.push(m_by);
            } else {
                commands.push(m_to);
            }
            end = segment.start;

            for command in &segment.data {
                // TODO: Pick best SVG path representation for each command
                todo!()
            }
        }
        crate::Path(commands)
    }
}
