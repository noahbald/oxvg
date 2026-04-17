use crate::{
    geometry::{Curve, Point},
    optimize::Options,
    paths::segment::{Data, Path, Tolerance},
};

impl Path {
    pub fn simplify(&mut self, options: Options, tolerance: &Tolerance) {
        let tolerance_squared = tolerance.square();
        if options.contains(Options::JoinNodes) {
            for segment in self.0.iter_mut() {
                for command in segment.data.iter_mut() {
                    match command {
                        Data::CurveTo(curve) => {
                            if curve.is_straight(&tolerance_squared) {
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

                        (Data::CurveTo(previous), Data::CurveTo(current))
                            if previous
                                .end_control()
                                .reflect(previous.end_point())
                                .distance_squared(&current.start_control())
                                < *tolerance_squared =>
                        {
                            *previous = Curve::new(
                                previous.start_control(),
                                current.end_control(),
                                current.end_point(),
                            );
                        }
                        (Data::ArcTo(previous), Data::ArcTo(current))
                            if previous.center().distance_squared(&current.center())
                                < *tolerance_squared
                                && previous.radii().distance_squared(&current.radii())
                                    < *tolerance_squared
                                && (previous.x_rotation() - current.x_rotation()).abs()
                                    < tolerance.angular =>
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
