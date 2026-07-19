//! Gilbert-Johnson-Keerthi algorithm implementation
use crate::{
    geometry::{self, Point},
    paths::segment::{Data, Path, Segment},
};

struct IndexBounds {
    source: Vec<Point>,
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
}

impl Path {
    /// Checks if two paths have an intersection by checking convex hulls collision using
    /// Gilbert-Johnson-Keerthi distance algorithm.
    ///
    /// # Panics
    ///
    /// If internal assertions fail
    #[allow(clippy::similar_names)]
    pub fn intersects(&self, other: &Self) -> bool {
        let self_hull: Vec<Vec<_>> = self
            .0
            .iter()
            .filter(|segment| !segment.data.is_empty())
            .map(hull_nest)
            .map(Iterator::collect)
            .collect();
        let other_hull: Vec<Vec<_>> = other
            .0
            .iter()
            .filter(|segment| !segment.data.is_empty())
            .map(hull_nest)
            .map(Iterator::collect)
            .collect();
        let (bbbox, sub_bbbox) = build_bbox(&self_hull);
        let (fbbox, sub_fbbox) = build_bbox(&other_hull);

        if bbbox.1.x <= fbbox.0.x
            || fbbox.1.x <= bbbox.0.x
            || bbbox.1.y <= fbbox.0.y
            || fbbox.1.y <= bbbox.0.y
            || sub_bbbox.iter().all(|bbbox| {
                sub_fbbox.iter().all(|fbbox| {
                    bbbox.1.x <= fbbox.0.x
                        || fbbox.1.x <= bbbox.0.x
                        || bbbox.1.y <= fbbox.0.y
                        || fbbox.1.y <= bbbox.0.y
                })
            })
        {
            log::debug!("no intersection, bounds check failed");
            return false;
        }

        // PERF: Smaller segment count should be `right` to avoid large allocation.
        let (left, right) = if self_hull.len() < other_hull.len() {
            (other_hull, self_hull)
        } else {
            (self_hull, other_hull)
        };
        let hull_nest_2: Vec<_> = IndexBounds::new_iter(right).collect(); // PERF: See, right is allocated

        IndexBounds::new_iter(left).any(|hull_1| {
            hull_nest_2.iter().any(|hull_2| {
                let mut simplex = vec![hull_1.get_support(hull_2, Point::X)];
                let mut direction = -simplex[0];

                for _ in 0..10_000 {
                    simplex.push(hull_1.get_support(hull_2, direction));
                    if direction.dot(*simplex.last().unwrap()) <= 0.0 {
                        return false;
                    }
                    if process_simplex(&mut simplex, &mut direction) {
                        return true;
                    }
                }
                log::error!("Infinite loop while finding path intersections");
                true
            })
        })
    }
}

impl IndexBounds {
    fn new(source: Vec<Point>) -> Self {
        let mut min_x = 0;
        let mut min_y = 0;
        let mut max_x = 0;
        let mut max_y = 0;
        for (i, point) in source.iter().enumerate() {
            if point.x < source[min_x].x {
                min_x = i;
            }
            if point.y < source[min_y].y {
                min_y = i;
            }
            if point.x > source[max_x].x {
                max_x = i;
            }
            if point.y > source[max_y].y {
                max_y = i;
            }
        }
        Self {
            source,
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    fn new_iter(hull_nests: Vec<Vec<Point>>) -> impl Iterator<Item = Self> {
        hull_nests.into_iter().map(Self::new).map(Self::convex_hull)
    }

    /// Gets the support point of the Minowski difference of two shapes.
    fn get_support(&self, other: &IndexBounds, direction: Point) -> geometry::Point {
        self.support_point(direction) - (other.support_point(-direction))
    }

    /// Get the supporting point of a polygon, the furthest point in a given direction.
    pub fn support_point(&self, direction: Point) -> Point {
        let mut index = if direction.y >= 0.0 {
            if direction.x < 0.0 {
                self.max_y
            } else {
                self.max_x
            }
        } else if direction.x < 0.0 {
            self.min_x
        } else {
            self.min_y
        };
        let mut max = -f64::INFINITY;
        loop {
            let value = self.source[index].dot(direction);
            if value <= max {
                break;
            }
            max = value;
            index = (index + 1) % self.source.len();
        }

        if index == 0 {
            index = self.source.len();
        }
        self.source[index - 1]
    }

    #[must_use]
    pub fn convex_hull(mut self) -> Self {
        self.source
            .sort_by(|geometry::Point(a), geometry::Point(b)| {
                if a.x == b.x {
                    a.y.total_cmp(&b.y)
                } else {
                    a.x.total_cmp(&b.x)
                }
            });

        let mut lower: Vec<Point> = vec![];
        let mut min_y = 0;
        let mut bottom = 0;

        for (i, point) in self.source.iter().enumerate() {
            while lower.len() >= 2
                && lower[lower.len() - 2].cross(lower[lower.len() - 1], *point) <= 0.0
            {
                lower.pop();
            }
            if point.y < self.source[min_y].y {
                min_y = i;
                bottom = lower.len();
            }
            lower.push(*point);
        }

        let mut upper: Vec<Point> = vec![];
        let mut max_y = self.source.len() - 1;
        let mut top = 0;

        for (i, point) in self.source.iter().enumerate().rev() {
            while upper.len() >= 2
                && upper[upper.len() - 2].cross(upper[upper.len() - 1], *point) <= 0.0
            {
                upper.pop();
            }
            if point.y > self.source[max_y].y {
                max_y = i;
                top = upper.len();
            }
            upper.push(*point);
        }

        upper.pop();
        lower.pop();

        let max_x = lower.len();
        lower.extend(upper);
        let max_y = if lower.is_empty() {
            0
        } else {
            (max_x + top) % lower.len()
        };

        Self {
            source: lower,
            min_x: 0,
            max_x,
            min_y: bottom,
            max_y,
        }
    }
}

fn build_bbox(hull: &[Vec<Point>]) -> ((Point, Point), Vec<(Point, Point)>) {
    let mut bbox = (Point::NAN, Point::NAN);
    let mut sub_bbox = vec![];

    for segment in hull {
        sub_bbox.push((Point::NAN, Point::NAN));
        for point in segment {
            let sub_bbox = sub_bbox.last_mut().unwrap();
            bbox.0.x = bbox.0.x.min(point.x);
            bbox.0.y = bbox.0.y.min(point.y);
            bbox.1.x = bbox.1.x.max(point.x);
            bbox.1.y = bbox.1.y.max(point.y);
            sub_bbox.0.x = sub_bbox.0.x.min(point.x);
            sub_bbox.0.y = sub_bbox.0.y.min(point.y);
            sub_bbox.1.x = sub_bbox.1.x.max(point.x);
            sub_bbox.1.y = sub_bbox.1.y.max(point.y);
        }
    }
    (bbox, sub_bbox)
}

/// Creates an iterator of points that cover the outer boundary of the given command.
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_precision_loss)]
fn hull_nest(segment: &Segment) -> impl Iterator<Item = Point> + use<'_> {
    std::iter::once(segment.start).chain(segment.data.iter().flat_map(|data| match data {
        Data::LineTo(point) => vec![*point],
        Data::CurveTo(curve) => vec![curve.start_control, curve.end_control, curve.end_point],
        Data::ArcTo(arc) => {
            let sweep = arc.sweep_angle();
            let rx = arc.radii().x;
            let ry = arc.radii().y;
            let center = arc.center();
            let x_rot = arc.x_rotation();

            let n = ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
            let step = sweep / n as f64;
            let k = (4.0 / 3.0) * (step / 4.0).tan();

            let point_at = |t: f64| {
                let (sin_t, cos_t) = t.sin_cos();
                Point::new(rx * cos_t, ry * sin_t).rotate_radian(x_rot) + center
            };
            let tangent_at = |t: f64| {
                let (sin_t, cos_t) = t.sin_cos();
                Point::new(-rx * sin_t, ry * cos_t).rotate_radian(x_rot)
            };

            let mut points = Vec::with_capacity(n * 3);
            for i in 0..n {
                let t1 = arc.start_angle() + step * i as f64;
                let t2 = t1 + step;
                let sub_end = point_at(t2);
                points.push(point_at(t1) + tangent_at(t1) * k);
                points.push(sub_end - tangent_at(t2) * k);
                points.push(sub_end);
            }
            points
        }
    }))
}

/// As part of the GJK algorithm, takes the current simplex (a polygon) and search direction
/// in order to find the direction and subset of the simplex to try next.
pub fn process_simplex(simplex: &mut Vec<Point>, direction: &mut Point) -> bool {
    // We only need to handle to 1-simplex and 2-simplex
    if simplex.len() == 2 {
        let a = simplex[1];
        let b = simplex[0];
        let ao = -a;
        let ab = b - a;
        // ao is in the same direction as ab
        if ao.dot(ab) > 0.0 {
            // get the vector perpendicular to ab facing o
            *direction = ab.orth(a);
        } else {
            *direction = ao;
            // only a remains in the simplex
            simplex.remove(0);
        }
    } else {
        // 2-simplex
        let a = simplex[2];
        let b = simplex[1];
        let c = simplex[0];
        let ab = b - a;
        let ac = c - a;
        let ao = -a;
        let acb = ab.orth(ac); // The vector perpendicular to ab facing away from c
        let abc = ac.orth(ab); // The vector perpendicular to ac facing away from b

        if acb.dot(ao) > 0.0 {
            if ab.dot(ao) > 0.0 {
                // region 4
                *direction = acb;
                simplex.remove(0);
            } else {
                // region 5
                *direction = ao;
                simplex.drain(0..=1);
            }
        } else if abc.dot(ao) > 0.0 {
            if ac.dot(ao) > 0.0 {
                // region 6
                *direction = abc;
                simplex.remove(1);
            } else {
                // region 5 (again)
                *direction = ao;
                simplex.drain(0..=1);
            }
        } else {
            return true;
        }
    }
    false
}
