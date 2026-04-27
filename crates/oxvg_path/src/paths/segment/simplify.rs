use crate::{
    geometry::{Curve, Point},
    optimize::Options,
    paths::segment::{Data, Path, Tolerance},
};

impl Path {
    pub fn simplify(&mut self, options: Options, tolerance: &Tolerance) {
        let tolerance_squared = tolerance.square();
        let mut segments = self.iter_start_cursor_mut();
        while let Some((start, command)) = segments.next() {
            match command {
                Data::CurveTo(curve) => {
                    if curve.is_straight(start, tolerance.positional) {
                        *command = Data::LineTo(curve.end_point())
                    }
                }
                Data::ArcTo(arc) => {
                    if arc.is_straight(tolerance.positional) {
                        *command = Data::LineTo(arc.end_point())
                    }
                }
                _ => {}
            }
        }

        if options.contains(Options::JoinNodes) {
            for segment in self.0.iter_mut() {
                let mut new_data = vec![];
                let mut start = segment.start;
                for command in segment.data.drain(..) {
                    let Some(previous) = new_data.last_mut() else {
                        new_data.push(command);
                        continue;
                    };

                    match (previous, &command) {
                        (Data::LineTo(previous), Data::LineTo(current))
                            if Point::cross(start, *previous, *current).abs()
                                < tolerance.positional =>
                        {
                            *previous = *current;
                        }

                        (Data::CurveTo(previous), Data::CurveTo(current)) => {
                            let left_tangent = previous.end_point() - previous.end_control();
                            let right_tangent = current.start_control() - previous.end_point();
                            let left_len = left_tangent.len();
                            let right_len = right_tangent.len();

                            let cross =
                                Point::cross(Point::ZERO, left_tangent, right_tangent).abs();
                            let are_parallel = cross < tolerance.positional;

                            if are_parallel && left_len + right_len > 0.0 {
                                let t = left_len / (left_len + right_len);
                                let p1 = start + (previous.start_control() - start) / t;
                                let p2 = current.end_point()
                                    + (current.end_control() - current.end_point()) / (1.0 - t);
                                *previous = Curve::new(p1, p2, current.end_point());
                            } else {
                                start = previous.end_point();
                                new_data.push(command);
                            }
                        }
                        (Data::ArcTo(previous), Data::ArcTo(current))
                            if previous.center().distance_squared(&current.center())
                                < *tolerance_squared
                                && previous.radii().distance_squared(&current.radii())
                                    < *tolerance_squared
                                && (
                                    // TOOD: Check x_rotation affects start_angle
                                    previous.radii().x() - previous.radii().y() < tolerance.angular
                                        || (previous.x_rotation() - current.x_rotation()).abs()
                                            < tolerance.angular
                                ) =>
                        {
                            let new_sweep = previous.sweep_angle() + current.sweep_angle();
                            previous.0[5] = new_sweep;
                        }
                        (previous, _) => {
                            // This command will be the start of the next set of joins.
                            // The previous end is the start of this command.
                            start = previous.end_point();
                            new_data.push(command);
                        }
                    }
                }
                segment.data = new_data;
            }
        }
        if options.contains(Options::RemoveEmptySegments) {
            self.0.retain(|segment| !segment.data.is_empty());
        }
        if options.contains(Options::RemoveCloseLine) {
            for segment in self.0.iter_mut().filter(|segment| segment.closed()) {
                if let Some(Data::LineTo(end)) = segment.data.last() {
                    if segment.start().distance_squared(end) < *tolerance_squared {
                        segment.data.pop();
                        segment.closed = true;
                    } else {
                        // Line end outside of tolerance; segment must have
                        // been coerced to `closed` in `Path::optimize`.
                        segment.closed = false;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::f64::consts::PI;

    use crate::{
        geometry::{Arc, Curve, Point},
        optimize::{Options, Tolerance},
        paths::segment::{Data, Path, Segment},
    };

    #[test]
    fn convert_straight_curves() {
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![
                Data::ArcTo(Arc::new(Point([1.0, 0.0]), Point([1.0, 0.0]), -PI, PI, 0.0)),
                Data::CurveTo(Curve::new(
                    Point([3.0, 0.0]),
                    Point([4.0, 0.0]),
                    Point([5.0, 0.0]),
                )),
            ],
            closed: false,
        }]);
        path.simplify(
            Options::empty(),
            &crate::paths::segment::Tolerance {
                positional: 0.01,
                angular: 0.01,
            },
        );
        assert_eq!(
            path,
            Path(vec![Segment {
                start: Point::ZERO,
                data: vec![
                    Data::LineTo(Point([2.0, 0.0])),
                    Data::LineTo(Point([5.0, 0.0]))
                ],
                closed: false,
            }])
        );
    }

    #[test]
    fn join_nodes() {
        let arc = Arc::new(Point([1.0, 1.0]), Point([1.0, 1.0]), -PI, PI, 0.0);
        let (arc1, arc2) = arc.subdivide();
        let curve = Curve::new(Point([2.0, 0.0]), Point([2.0, 0.0]), Point([4.0, 1.0]));
        let (curve1, _, curve2) = curve.subdivide_t(Point([2.0, 1.0]), 0.25);
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![
                Data::ArcTo(arc1),
                Data::ArcTo(arc2),
                Data::CurveTo(curve1),
                Data::CurveTo(curve2),
                Data::LineTo(Point([5.0, 1.0])),
                Data::LineTo(Point([6.0, 1.0])),
            ],
            closed: false,
        }]);
        path.simplify(Options::JoinNodes, &Tolerance::default());
        assert_eq!(
            path,
            Path(vec![Segment {
                start: Point::ZERO,
                data: vec![
                    Data::ArcTo(arc),
                    Data::CurveTo(curve),
                    Data::LineTo(Point([6.0, 1.0])),
                ],
                closed: false,
            }])
        )
    }

    #[test]
    fn remove_empty_segments() {
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![],
            closed: false,
        }]);
        path.simplify(Options::RemoveEmptySegments, &Tolerance::default());
        assert_eq!(path, Path(vec![]))
    }

    #[test]
    fn remove_close_line() {
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![
                Data::LineTo(Point([1.0, 0.0])),
                Data::LineTo(Point([1.0, 1.0])),
                Data::LineTo(Point::ZERO),
            ],
            closed: true,
        }]);
        path.simplify(Options::RemoveCloseLine, &Tolerance::default());
        assert_eq!(
            path,
            Path(vec![Segment {
                start: Point::ZERO,
                data: vec![
                    Data::LineTo(Point([1.0, 0.0])),
                    Data::LineTo(Point([1.0, 1.0])),
                ],
                closed: true,
            }])
        )
    }
}
