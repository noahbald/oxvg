//! Gilbert-Johnson-Keerthi algorithm implementation
use crate::{
    convert, geometry,
    points::{Point, Points},
    Path,
};

impl Path {
    /// Checks if two paths have an intersection by checking convex hulls collision using
    /// Gilbert-Johnson-Keerthi distance algorithm.
    ///
    /// # Panics
    /// If internal assertions fail
    pub fn intersects(&self, other: &Self) -> bool {
        let points_1 = Points::from_positioned(&convert::relative(self.clone()));
        let points_2 = Points::from_positioned(&convert::relative(other.clone()));

        // First check whether their bounding box intersects
        if points_1.max_x <= points_2.min_x
            || points_2.max_x <= points_1.min_x
            || points_1.max_y <= points_2.min_y
            || points_2.max_y <= points_1.min_y
            || points_1.list.iter().all(|set_1| {
                points_2.list.iter().all(|set_2| {
                    set_1.list[set_1.max_x].0[0] <= set_2.list[set_2.min_x].0[0]
                        || set_2.list[set_2.max_x].0[0] <= set_1.list[set_1.min_x].0[0]
                        || set_1.list[set_1.max_y].0[1] <= set_2.list[set_2.min_y].0[1]
                        || set_2.list[set_2.max_y].0[1] <= set_1.list[set_1.min_y].0[1]
                })
            })
        {
            log::debug!("no intersection, bounds check failed");
            return false;
        }

        // i.e. https://en.wikipedia.org/wiki/Gilbert%E2%80%93Johnson%E2%80%93Keerthi_distance_algorithm
        let mut hull_nest_1 = points_1.list.into_iter().map(Point::convex_hull);
        let hull_nest_2: Vec<_> = points_2.list.into_iter().map(Point::convex_hull).collect();

        hull_nest_1.any(|hull_1| {
            if hull_1.list.len() < 3 {
                return false;
            }

            hull_nest_2.iter().any(|hull_2| {
                if hull_2.list.len() < 3 {
                    return false;
                }

                let mut simplex = vec![hull_1.get_support(hull_2, geometry::Point([1.0, 0.0]))];
                let mut direction = simplex[0].minus();
                let mut iterations = 10_000;

                loop {
                    iterations -= 1;
                    if iterations == 0 {
                        log::error!("Infinite loop while finding path intersections");
                        return true;
                    }
                    simplex.push(hull_1.get_support(hull_2, direction));
                    if direction.dot(simplex.last().unwrap()) <= 0.0 {
                        return false;
                    }
                    if geometry::Point::process_simplex(&mut simplex, &mut direction) {
                        return true;
                    }
                }
            })
        })
    }
}

impl geometry::Point {
    /// As part of the GJK algorithm, takes the current simplex (a polygon) and search direction
    /// in order to find the direction and subset of the simplex to try next.
    // TODO: Move this to intersection module?
    pub fn process_simplex(simplex: &mut Vec<Self>, Self(direction): &mut Self) -> bool {
        // We only need to handle to 1-simplex and 2-simplex
        if simplex.len() == 2 {
            let a = simplex[1];
            let b = simplex[0];
            let ao = a.minus();
            let ab = b - a;
            // ao is in the same direction as ab
            if ao.dot(&ab) > 0.0 {
                // get the vector perpendicular to ab facing o
                *direction = ab.orth(&a).0;
            } else {
                *direction = ao.0;
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
            let ao = a.minus();
            let acb = ab.orth(&ac); // The vector perpendicular to ab facing away from c
            let abc = ac.orth(&ab); // The vector perpendicular to ac facing away from b

            if acb.dot(&ao) > 0.0 {
                if ab.dot(&ao) > 0.0 {
                    // region 4
                    *direction = acb.0;
                    simplex.remove(0);
                } else {
                    // region 5
                    *direction = ao.0;
                    simplex.drain(0..=1);
                }
            } else if abc.dot(&ao) > 0.0 {
                if ac.dot(&ao) > 0.0 {
                    // region 6
                    *direction = abc.0;
                    simplex.remove(1);
                } else {
                    // region 5 (again)
                    *direction = ao.0;
                    simplex.drain(0..=1);
                }
            } else {
                return true;
            }
        }
        false
    }
}
