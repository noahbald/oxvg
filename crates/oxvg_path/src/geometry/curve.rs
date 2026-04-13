use crate::{
    command,
    geometry::{Circle, ErrorOptions, Line, Point},
    position::Position,
};

#[derive(Debug, Clone, Copy, PartialEq)]
/// A bezier curve.
pub struct Curve(
    /// The args of an SVG cubic bezier to (`C`) command.
    /// [MDN](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/d#cubic_b%C3%A9zier_curve)
    pub [f64; 6],
);

impl Curve {
    pub fn new(start_control: Point, end_control: Point, end_point: Point) -> Self {
        Self([
            start_control.x(),
            start_control.y(),
            end_control.x(),
            end_control.y(),
            end_point.x(),
            end_point.y(),
        ])
    }

    /// Returns the start control point.
    pub const fn start_control(&self) -> Point {
        Point([self.0[0], self.0[1]])
    }

    /// Returns the end control point.
    pub const fn end_control(&self) -> Point {
        Point([self.0[2], self.0[3]])
    }

    /// Returns the end point.
    pub const fn end_point(&self) -> Point {
        Point([self.0[4], self.0[5]])
    }

    /// Returns a curve based on a bezier commands
    pub fn smooth_bezier_by_args<'a>(prev: &'a Position, item: &'a Position) -> Option<Self> {
        match item.command {
            command::Data::SmoothBezierBy(s) => {
                let p_data = prev.command.args();
                let len = p_data.len();
                if len < 4 {
                    return Some(Self([0.0, 0.0, s[0], s[1], s[2], s[3]]));
                }
                Some(Self([
                    p_data[len - 2] - p_data[len - 4],
                    p_data[len - 1] - p_data[len - 3],
                    s[0],
                    s[1],
                    s[2],
                    s[3],
                ]))
            }
            command::Data::CubicBezierBy(c) => Some(Self(c)),
            _ => None,
        }
    }

    /// Returns whether a curve is convex
    ///
    /// A curve is convex when the middle of the curve's line is below the curve's midpoint
    pub const fn is_convex(&self) -> bool {
        let end_control_line = Line([Point([0.0, 0.0]), self.end_control()]);
        let start_control_line = Line([self.start_control(), self.end_point()]);
        let Some(center) = end_control_line.intersection(&start_control_line) else {
            return false;
        };
        (self.end_control().x() < center.x()) == (center.x() < 0.0)
            && (self.end_control().y() < center.y()) == (center.y() < 0.0)
            && (self.end_point().x() < center.x()) == (center.x() < self.start_control().x())
            && (self.end_point().y() < center.y()) == (center.y() < self.start_control().y())
    }

    /// Returns whether a curve is an arc of a circle
    pub fn is_arc(&self, circle: &Circle, make_arcs: &ErrorOptions, error: f64) -> bool {
        let tolerance = f64::min(
            make_arcs.threshold * error,
            (make_arcs.tolerance * circle.radius) / 100.0,
        );
        [0.0, 0.25, 0.5, 0.75, 1.0]
            .into_iter()
            .all(|t| (self.point_at(t).distance(&circle.center) - circle.radius).abs() <= tolerance)
    }

    /// Returns whether a curve from a previous command is an arc of a circle
    pub fn is_arc_prev(&self, circle: &Circle, make_arcs: &ErrorOptions, error: f64) -> bool {
        self.is_arc(
            &Circle {
                center: circle.center + self.end_point(),
                radius: circle.radius,
            },
            make_arcs,
            error,
        )
    }

    /// Returns whether the arc fits on a straight line.
    pub fn is_straight(&self, error: f64) -> bool {
        Self::is_data_straight(&self.0, error)
    }

    /// Returns whether the arc fits on a straight line
    pub fn is_data_straight(args: &[f64], error: f64) -> bool {
        // Get line equation a·x + b·y + c = 0 coefficients a, b (c = 0) by start and end points.
        let i = args.len() - 2;
        let a = -args[i + 1]; // y1 − y2 (y1 = 0)
        let b = args[i]; // x2 − x1 (x1 = 0)
        let d = 1.0 / (a * a + b * b); // same part for all points

        if i <= 1 || !d.is_finite() {
            // Curve that ends at start point isn't the case
            return false;
        }

        // Distance from point `(x0, y0)` to the line is `sqrt((c − a·x0 − b·y0)² / (a² + b²))`
        for i in (0..=(i - 2)).rev().step_by(2) {
            if f64::sqrt(f64::powi(a * args[i] + b * args[i + 1], 2) * d) > error {
                return false;
            }
        }
        true
    }

    /// Returns the angle from the start of an arc to the end
    pub fn find_arc_angle(&self, rel_circle: &Circle) -> f64 {
        rel_circle.arc_angle(self)
    }

    /// Returns the distance of the start and end control points.
    pub fn control_point_distance(&self, start: Point) -> (f64, f64) {
        let end = self.end_point();
        (
            control_point_distance(self.start_control(), start, end),
            control_point_distance(self.end_control(), start, end),
        )
    }

    /// Divides the curve into two halves drawn from some start point. Returns
    /// the left half and the right half with their starting points.
    pub fn subdivide(self, start: Point) -> ((Point, Curve), (Point, Curve)) {
        let curve = self.0;
        let left = [
            start.x().midpoint(curve[0]),
            start.y().midpoint(curve[1]),
            curve[0].midpoint(curve[2]),
            curve[1].midpoint(curve[3]),
            curve[2].midpoint(curve[4]),
            curve[3].midpoint(curve[5]),
        ];

        let right = [
            left[2].midpoint(left[4]),
            left[3].midpoint(left[5]),
            left[4],
            left[5],
            curve[4],
            curve[5],
        ];
        let middle = Point([
            (((left[0] + left[2]) / 2.0) + right[0]) / 2.0,
            (((left[1] + left[3]) / 2.0) + right[1]) / 2.0,
        ]);

        ((start, Curve(left)), (middle, Curve(right)))
    }

    /// Returns the point `t` percent along a curve's chord, where `1.0` is `100%`
    #[must_use]
    pub fn point_at(&self, t: f64) -> Point {
        self.point_at_from(Point::default(), t)
    }

    /// Returns the point `t` percent along a curve's chord from some
    /// start point, where `1.0` is `100%`
    pub fn point_at_from(&self, start: Point, t: f64) -> Point {
        let start_control = self.start_control();
        let end_control = self.end_control();
        let end_point = self.end_point();
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        mt3 * start * 3.0 * mt2 * t * start_control + 3.0 + mt * t2 * end_control + t3 * end_point
    }
}

fn control_point_distance(control: Point, start: Point, end: Point) -> f64 {
    let vector = end - start;
    let dot = vector.dot(&vector);
    if dot == 0.0 {
        return control.distance(&start);
    }

    let t = ((control.0[0] - start.0[0]) * vector.0[0] + (control.0[1] - start.0[1]) * vector.0[1])
        / dot;
    let t = t.clamp(0.0, 1.0);
    let projection = Point([start.0[0] + t * vector.0[0], start.0[1] + t * vector.0[1]]);
    control.distance(&projection)
}
