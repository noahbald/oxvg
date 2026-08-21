use std::f64::consts::PI;

use crate::{
    command::{self, ID},
    geometry::{Arc, Curve, Point, Tolerance, TolerancePrecision, ToleranceSquared},
    optimize::Options,
    paths::segment::{Data, IterStartCursorItem, Path, convert::compactest},
};

impl Path {
    /// Simplifies a path by cleaning up and minifying it's data.
    ///
    /// See [`Options`] for controls over the simplification operations.
    pub fn simplify(&mut self, options: Options, tolerance: &Tolerance) {
        let tolerance_squared = tolerance.square();
        let precision = tolerance.precision();

        if options.contains(Options::StraightCurves) {
            self.straight_curves(tolerance, tolerance_squared);
        }
        if options.contains(Options::ArcCurves) {
            self.arc_curves(
                tolerance,
                tolerance_squared,
                precision,
                options.contains(Options::SmartArcRounding),
            );
        }
        if options.contains(Options::RemoveNoopCommands) {
            self.remove_noop_commands(tolerance_squared);
        }
        if options.contains(Options::JoinNodes) {
            self.join_nodes(tolerance, tolerance_squared);
        }
        if options.contains(Options::RemoveCloseLine) {
            self.remove_close_line(tolerance_squared);
        }
        if options.contains(Options::RemoveEmptySegments) {
            self.remove_empty_segments();
        }
    }

    fn straight_curves(&mut self, tolerance: &Tolerance, tolerance_squared: ToleranceSquared) {
        let mut segments = self.iter_start_cursor_mut();
        let mut is_previous_quadratic = false;
        let mut last_control = None;
        while let Some(IterStartCursorItem {
            cursor: start,
            data,
            command,
            ..
        }) = segments.next()
        {
            let control = last_control.take();
            let Some(data) = data else {
                continue;
            };
            let previous_quadratic = command > 0 && is_previous_quadratic;
            is_previous_quadratic = false;
            match data {
                Data::CurveTo(curve) => {
                    // NOTE: 1: Don't straighten `t`/`T` continuing a chain.
                    //       See (2)
                    let quad_control = curve.quad_control(start, tolerance_squared);
                    let is_optimal = !previous_quadratic
                        || quad_control.is_none_or(|quad_control| {
                            curve
                                .smooth_quadratic_bezier_unchecked_quad(
                                    start,
                                    control,
                                    quad_control,
                                    tolerance_squared,
                                )
                                .is_none()
                        });
                    if is_optimal && curve.is_straight(start, tolerance) {
                        *data = Data::LineTo(curve.end_point);
                    } else {
                        // NOTE: 2: When quadratic is not straightened, it it
                        //       part of a chain
                        is_previous_quadratic = quad_control.is_some();
                    }
                    last_control = quad_control;
                }
                Data::ArcTo(arc) => {
                    if arc.is_straight(tolerance) {
                        *data = Data::LineTo(arc.end_point());
                    }
                }
                Data::LineTo(_) => {}
            }
        }
    }

    fn arc_curves(
        &mut self,
        tolerance: &Tolerance,
        tolerance_squared: ToleranceSquared,
        precision: TolerancePrecision,
        smart_arc_rounding: bool,
    ) {
        let candidates: Vec<_> = self
            .iter_start_cursor()
            .map(
                |IterStartCursorItem {
                     cursor: start,
                     data,
                     ..
                 }| {
                    match data? {
                        Data::CurveTo(curve) => {
                            Arc::fit_curve(curve, start, tolerance, tolerance_squared)
                        }
                        Data::ArcTo(arc) => Some(arc.clone()),
                        Data::LineTo(_) => None,
                    }
                },
            )
            .collect();

        let mut segments = self.iter_start_cursor_mut();
        let mut previous: Option<command::Data> = None;
        let mut last_control = None;
        let mut next_i = 0;
        while let Some(IterStartCursorItem {
            cursor: start,
            data,
            next,
            ..
        }) = segments.next()
        {
            next_i += 1;
            let Some(data) = data else {
                continue;
            };
            let implicit = previous.as_ref().map(|p| p.id().next_implicit());
            if let Data::CurveTo(curve) = data {
                let control = last_control.take();
                let (c, control) = Path::to_svg_curve(
                    previous.as_ref(),
                    control,
                    start,
                    curve,
                    next.as_deref(),
                    implicit.as_ref(),
                    tolerance_squared,
                    precision,
                );
                if let Some(arc) = candidates.get(next_i - 1).cloned().flatten() {
                    // First case: arc will be joined to next arc
                    if candidates
                        .get(next_i)
                        .cloned()
                        .flatten()
                        .is_some_and(|next| arc.is_connected(&next, tolerance, tolerance_squared))
                    {
                        *data = Data::ArcTo(arc);
                        continue;
                    }
                    // Second case: arc will be joined to previous arc
                    if next_i > 1
                        && candidates
                            .get(next_i - 2)
                            .cloned()
                            .flatten()
                            .is_some_and(|prev| {
                                prev.is_connected(&arc, tolerance, tolerance_squared)
                            })
                    {
                        *data = Data::ArcTo(arc);
                        continue;
                    }
                    // Third case: arc is optimal in comparison to curve
                    let a = Path::to_svg_arc(
                        previous.as_ref(),
                        &arc,
                        start,
                        implicit.as_ref(),
                        tolerance,
                        tolerance_squared,
                        precision,
                        smart_arc_rounding,
                    );
                    let c = compactest(previous.as_ref(), c, a, implicit.as_ref(), precision);
                    if matches!(c.id(), ID::ArcBy | ID::ArcTo) {
                        *data = Data::ArcTo(arc);
                        continue;
                    }
                    previous = Some(c);
                }

                last_control = control;
            } else {
                previous = None;
            }
        }
    }

    fn remove_noop_commands(&mut self, tolerance_squared: ToleranceSquared) {
        for segment in &mut self.0 {
            let mut start = segment.start;
            segment.data.retain(|command| {
                let is_zero = match command {
                    Data::LineTo(p) => p.distance_squared(start) < *tolerance_squared,
                    Data::CurveTo(curve) => {
                        curve.start_control.distance_squared(start) < *tolerance_squared
                            && curve.end_control.distance_squared(start) < *tolerance_squared
                            && curve.end_point.distance_squared(start) < *tolerance_squared
                    }
                    Data::ArcTo(arc) => {
                        arc.sweep_angle().abs() < PI
                            && arc.end_point().distance_squared(start) < *tolerance_squared
                    }
                };
                start = command.end_point();
                !is_zero
            });
        }
        if let Some(segment) = self.0.last_mut()
            && segment.closed
        {
            segment.data.pop_if(|command| match command {
                Data::LineTo(p) => p.distance_squared(segment.start) < *tolerance_squared,
                _ => false,
            });
        }
    }

    #[allow(clippy::too_many_lines)]
    fn join_nodes(&mut self, tolerance: &Tolerance, tolerance_squared: ToleranceSquared) {
        for segment in &mut self.0 {
            let mut new_data = vec![];
            let mut start = segment.start;
            for command in segment.data.drain(..) {
                let Some(previous) = new_data.last_mut() else {
                    new_data.push(command);
                    continue;
                };

                match (previous, &command) {
                    (Data::LineTo(previous), Data::LineTo(current))
                        if Point::is_continuous_parallel(start, *previous, *current, tolerance) =>
                    {
                        *previous = *current;
                    }

                    (Data::CurveTo(previous), Data::CurveTo(current)) => {
                        if Point::is_continuous_parallel(
                            previous.end_control,
                            previous.end_point,
                            current.start_control,
                            tolerance,
                        ) {
                            let d1 = previous.end_point.distance(previous.end_control);
                            let d2 = current.start_control.distance(previous.end_point);
                            let sum = d1 + d2;

                            let t = if sum > 1e-9 {
                                d1 / sum
                            } else {
                                let left_len = previous.end_point.distance(start);
                                let right_len = current.end_point.distance(previous.end_point);
                                let chord_sum = left_len + right_len;
                                if chord_sum > 1e-9 {
                                    left_len / chord_sum
                                } else {
                                    0.5
                                }
                            };

                            let p1 = (previous.start_control - start * (1.0 - t)) / t;
                            let p2 = (current.end_control - current.end_point * t) / (1.0 - t);
                            let merged = Curve::new(p1, p2, current.end_point);

                            let (left, split_point, right) = merged.subdivide_t(start, t);
                            if split_point.distance_squared(previous.end_point) < *tolerance_squared
                                && left.end_control.distance_squared(previous.end_control)
                                    < *tolerance_squared
                                && right.start_control.distance_squared(current.start_control)
                                    < *tolerance_squared
                            {
                                *previous = merged;
                            } else {
                                start = previous.end_point;
                                new_data.push(command);
                            }
                        } else {
                            start = previous.end_point;
                            new_data.push(command);
                        }
                    }
                    (Data::ArcTo(previous), Data::ArcTo(current))
                        if previous.is_connected(current, tolerance, tolerance_squared) =>
                    {
                        let prev_sweep = previous.sweep_angle();
                        let current_sweep = current.sweep_angle();
                        let previous_len = previous.len(4);
                        let current_len = current.len(4);
                        let r_prev = previous.radii().x.midpoint(previous.radii().y);
                        let r_curr = current.radii().x.midpoint(current.radii().y);
                        if previous_len >= current_len {
                            // Fit current arc deviation onto previous's ellipse
                            let scale = r_curr / r_prev.max(1e-9);
                            let converted = current_sweep * scale;
                            previous.set_sweep_angle(previous.sweep_angle() + converted);
                            if previous.is_circle(tolerance) {
                                let r1 = previous.center().distance(start);
                                let r2 = previous.center().distance(current.end_point());
                                let avg_r = r1.midpoint(r2);
                                previous.set_radii(Point::splat(avg_r));
                            }
                            previous.set_end_point_memo(current.end_point());
                        } else {
                            let mut current = current.clone();
                            let end_point = current.end_point();
                            // Fit previous arc deviation onto current's ellipse
                            let scale = r_prev / r_curr.max(1e-9);
                            let converted = prev_sweep * scale;
                            current.set_start_angle(current.start_angle() - converted);
                            current.set_sweep_angle(current.sweep_angle() + converted);
                            if current.is_circle(tolerance) {
                                let r1 = current.center().distance(start);
                                let r2 = current.center().distance(end_point);
                                let avg_r = r1.midpoint(r2);
                                current.set_radii(Point::splat(avg_r));
                            }
                            current.set_end_point_memo(end_point);
                            *previous = current;
                        }
                        if previous.sweep_angle().abs() > 2.0 * PI - tolerance.angular
                            || previous.end_point() == start
                        {
                            let delta = PI.copysign(previous.sweep_angle());
                            previous.set_sweep_angle(delta);
                            let mut next = previous.clone();
                            next.set_start_angle(previous.start_angle() + delta);
                            next.set_sweep_angle(delta);
                            next.set_end_point_memo(current.end_point());
                            previous.set_end_point_memo(previous.end_point());
                            start = previous.end_point();
                            new_data.push(Data::ArcTo(next));
                        }
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

    fn remove_close_line(&mut self, tolerance_squared: ToleranceSquared) {
        for segment in self.0.iter_mut().filter(|segment| segment.closed()) {
            if let Some(end) = segment.data.last().map(|data| match data {
                Data::LineTo(end) => *end,
                Data::CurveTo(curve) => curve.end_point,
                Data::ArcTo(arc) => arc.end_point(),
            }) {
                if segment.start().distance_squared(end) < *tolerance_squared {
                    segment.closed = segment
                        .data
                        .pop_if(|command| matches!(command, Data::LineTo(_)))
                        .is_some();
                } else {
                    // Line end outside of tolerance; segment must have
                    // been coerced to `closed` in `Path::optimize`.
                    segment.closed = false;
                }
            } else {
                segment.closed = false;
            }
        }
    }

    fn remove_empty_segments(&mut self) {
        self.0.retain(|segment| !segment.data.is_empty());
    }
}

#[cfg(test)]
mod test {
    use std::f64::consts::PI;

    use crate::{
        geometry::{Arc, Curve, Point, Tolerance},
        optimize::Options,
        paths::{
            segment::{Data, Path, Segment},
            svg,
        },
    };

    #[test]
    fn convert_straight_curves() {
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![
                Data::ArcTo(Arc::new(
                    Point::new(1.0, 0.0),
                    Point::new(1.0, 0.0),
                    -PI,
                    PI,
                    0.0,
                )),
                Data::CurveTo(Curve::new(
                    Point::new(3.0, 0.0),
                    Point::new(4.0, 0.0),
                    Point::new(5.0, 0.0),
                )),
            ],
            closed: false,
        }]);
        path.simplify(
            Options::StraightCurves,
            &crate::geometry::Tolerance {
                positional: 0.01,
                angular: 0.01,
                precision: 2,
            },
        );
        assert_eq!(
            path,
            Path(vec![Segment {
                start: Point::ZERO,
                data: vec![
                    Data::LineTo(Point::new(2.0, 0.0)),
                    Data::LineTo(Point::new(5.0, 0.0))
                ],
                closed: false,
            }])
        );
    }

    #[test]
    fn join_nodes() {
        let arc = Arc::new(Point::new(1.0, 0.0), Point::new(1.0, 1.0), -PI, PI, 0.0)
            .with_end_point_memo(Point::new(2.0, 0.0));
        let (arc1, arc2) = arc.subdivide();
        let curve = Curve::new(
            Point::new(2.0, -1.0),
            Point::new(2.0, -1.0),
            Point::new(4.0, 0.0),
        );
        let (curve1, _, curve2) = curve.subdivide_t(Point::new(2.0, 0.0), 0.25);
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![
                Data::ArcTo(arc1),
                Data::ArcTo(arc2),
                Data::CurveTo(curve1),
                Data::CurveTo(curve2),
                Data::LineTo(Point::new(5.0, 0.0)),
                Data::LineTo(Point::new(6.0, 0.0)),
            ],
            closed: false,
        }]);
        path.simplify(Options::JoinNodes, &Tolerance::default());

        assert_eq!(
            path.to_svg(&Tolerance::default(), true).to_string(),
            "M0 0a.5.5 0 0 1 2 0c0-1 0-1 2 0h2"
        );
    }

    #[test]
    fn join_adjacent_arcs_non_continuous() {
        // Don't join; arcs are relatively distinct
        let svg_path = svg::Path(vec![
            svg::command::Data::MoveTo([0.0, 0.0]),
            svg::command::Data::ArcTo([4.0, 4.0, 0.0, 0.0, 0.0, -1.5, -5.0]),
            svg::command::Data::ArcTo([4.0, 4.0, 0.0, 0.0, 0.0, 0.2, -7.3]),
        ]);

        let mut path = Path::from_svg(&svg_path, &Tolerance::default());
        path.simplify(Options::JoinNodes, &Tolerance::default());

        assert_eq!(
            path.to_svg(&Tolerance::default(), true).to_string(),
            "M0 0a4 4 0 0 0-1.5-5A4 4 0 0 0 .2-7.3"
        );
    }

    #[test]
    fn join_tiny_adjacent_arcs_non_continuous() {
        // Don't join; distant center point creates a kinked end
        let svg_path = svg::Path(vec![
            svg::command::Data::MoveTo([0.0, 0.0]),
            svg::command::Data::ArcTo([4.0, 4.0, 0.0, 0.0, 0.0, -1.5, -5.0]),
            svg::command::Data::ArcBy([4.0, 4.0, 0.0, 0.0, 0.0, -1.5e-6, -5e-6]),
        ]);

        let mut path = Path::from_svg(&svg_path, &Tolerance::default());
        path.simplify(Options::JoinNodes, &Tolerance::default());

        assert_eq!(
            path.to_svg(&Tolerance::default(), true).to_string(),
            "M0 0a4 4 0 0 0-1.5-5 4 4 0 0 0 0-.001"
        );
    }

    #[test]
    fn join_tiny_adjacent_arcs_non_continuous_2() {
        // Don't join; distant center point creates a kinked end
        let svg_path = svg::Path(vec![
            svg::command::Data::MoveTo([84.331, 69.174]),
            svg::command::Data::ArcBy([14.48, 14.48, 0.0, 0.0, 1.0, -0.819, 4.425]),
            svg::command::Data::ArcBy([0.478, 0.478, 0.0, 0.0, 0.0, -0.024, 0.147]),
        ]);

        let mut path = Path::from_svg(&svg_path, &Tolerance::default());
        path.simplify(Options::JoinNodes, &Tolerance::default());

        assert_eq!(
            path.to_svg(&Tolerance::default(), true).to_string(),
            "M84.331 69.174a14.5 14.5 0 0 1-.819 4.425.5.5 0 0 0-.024.147"
        );
    }

    #[test]
    fn join_tiny_adjacent_arcs_continuous() {
        // Do join; distant radius has no effect on continuity
        let svg_path = svg::Path(vec![
            svg::command::Data::MoveTo([0.0, 0.0]),
            svg::command::Data::ArcTo([4.0, 4.0, 0.0, 0.0, 0.0, -1.5, -5.0]),
            svg::command::Data::ArcBy([1.0e3, 1.0e3, 0.0, 0.0, 0.0, -6e-6, -5e-6]),
        ]);

        let mut path = Path::from_svg(&svg_path, &Tolerance::default());
        path.simplify(Options::JoinNodes, &Tolerance::default());

        assert_eq!(
            path.to_svg(&Tolerance::default(), true).to_string(),
            "M0 0a4 4 0 0 0-1.5-5 1000 1000 0 0 0-.001-.001"
        );
    }

    #[test]
    fn join_tiny_adjacent_arcs_complex() {
        let svg_path = svg::Path(vec![
            svg::command::Data::MoveTo([15.0, 23.54]),
            svg::command::Data::CubicBezierBy([-2.017, 0.0, -3.87, -0.7, -5.33, -1.87]),
            svg::command::Data::CubicBezierBy([-0.032, -0.023, -0.068, -0.052, -0.11, -0.087]),
            svg::command::Data::CubicBezierBy([0.042, 0.035, 0.078, 0.064, 0.11, 0.087]),
            svg::command::Data::CubicBezierBy([0.048, 0.04, 0.08, 0.063, 0.08, 0.063]),
        ]);

        let mut path = Path::from_svg(&svg_path, &Tolerance::default());
        path.simplify(
            Options::JoinNodes | Options::ArcCurves | Options::SmartArcRounding,
            &Tolerance::default(),
        );

        assert_eq!(
            path.to_svg(&Tolerance::default(), true).to_string(),
            "M15 23.54a8.5 8.5 0 0 1-5.44-1.957 2 2 0 0 0 .19.15"
        );
    }

    #[test]
    fn join_tiny_adjacent_arcs_complex_2() {
        let svg_path = svg::Path(vec![
            svg::command::Data::MoveTo([956.5625, -57.34375]),
            svg::command::Data::CubicBezierBy([
                956.54344,
                -57.401_145,
                956.52483,
                -57.438_965,
                956.5,
                -57.5,
            ]),
            svg::command::Data::CubicBezierBy([
                955.80665,
                -59.204_045,
                953.10185,
                -64.396_096,
                951.75,
                -66.03125,
            ]),
        ]);

        let mut path = Path::from_svg(&svg_path, &Tolerance::default());
        path.simplify(
            Options::JoinNodes | Options::ArcCurves | Options::SmartArcRounding,
            &Tolerance::default(),
        );

        assert_eq!(
            path.to_svg(&Tolerance::default(), true).to_string(),
            "M956.563-57.344a817863 817863 0 0 0 956.5-57.5 14866 14866 0 0 0 951.75-66.031"
        );
    }

    #[test]
    fn remove_empty_segments() {
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![],
            closed: false,
        }]);
        path.simplify(Options::RemoveEmptySegments, &Tolerance::default());
        assert_eq!(path, Path(vec![]));
    }

    #[test]
    fn remove_close_line() {
        let mut path = Path(vec![Segment {
            start: Point::ZERO,
            data: vec![
                Data::LineTo(Point::new(1.0, 0.0)),
                Data::LineTo(Point::new(1.0, 1.0)),
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
                    Data::LineTo(Point::new(1.0, 0.0)),
                    Data::LineTo(Point::new(1.0, 1.0)),
                ],
                closed: true,
            }])
        );
    }
}
