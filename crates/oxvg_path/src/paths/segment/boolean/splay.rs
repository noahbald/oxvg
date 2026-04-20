use std::{cmp::Ordering, ops::Deref, rc::Rc};

use crate::{
    geometry::{Intersection, Line},
    paths::segment::boolean::{
        sweep_event::SweepEvent,
        utils::{inverse_less_if, less_if, signed_area},
    },
};

#[derive(Debug, Clone)]
pub struct Entry(pub Rc<SweepEvent>);

impl Deref for Entry {
    type Target = Rc<SweepEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for Entry {}
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(self, other)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        debug_assert!(self.left());
        debug_assert!(other.left());
        debug_assert!(self.other().is_some());
        debug_assert!(other.other().is_some());

        if Rc::ptr_eq(self, other) {
            return Ordering::Equal;
        }

        let (old_left, new_left, less_if): (
            &Rc<SweepEvent>,
            &Rc<SweepEvent>,
            fn(bool) -> Ordering,
        ) = if self.is_before(other) {
            (self, other, less_if)
        } else {
            (other, self, inverse_less_if)
        };

        let old_right = old_left.other().unwrap();
        let new_right = new_left.other().unwrap();
        let left_area = signed_area(old_left.point, old_right.point, new_left.point);
        let right_area = signed_area(old_left.point, old_right.point, new_right.point);
        if left_area != 0.0 || right_area != 0.0 {
            if old_left.point == new_left.point {
                return less_if(old_left.is_below(new_right.point));
            } else if old_left.point.x() == new_left.point.x() {
                return less_if(old_left.point.y() < new_left.point.y());
            } else if (left_area > 0.0) == (right_area > 0.0) {
                return less_if(left_area > 0.0);
            } else if left_area == 0.0 {
                return less_if(right_area > 0.0);
            }
            match Line([old_left.point, old_right.point])
                .intersection(&Line([new_left.point, new_right.point]))
            {
                Intersection::None => return less_if(left_area > 0.0),
                Intersection::Intersection(p) => {
                    if p == new_left.point {
                        return less_if(right_area > 0.0);
                    } else {
                        return less_if(left_area > 0.0);
                    }
                }
                Intersection::Parallel(..) => {}
            }
        }

        if old_left.source.background == new_left.source.background {
            if old_left.point == new_left.point {
                less_if(old_left.contour_id < new_left.contour_id)
            } else {
                less_if(true)
            }
        } else {
            less_if(old_left.source.background)
        }
    }
}

pub type Set = splay_tree::SplaySet<Entry>;

#[cfg(test)]
mod test {
    use std::{
        cmp::Ordering,
        rc::{Rc, Weak},
    };

    use super::*;
    use crate::{
        geometry::Point,
        paths::segment::boolean::sweep_event::{Source, SweepEvent},
    };

    macro_rules! assert_ordering {
        ($se1:expr, $se2:expr, $ordering:expr) => {
            let inverse_ordering = match $ordering {
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
                _ => Ordering::Equal,
            };
            assert_eq!(
                $se1.cmp($se2),
                $ordering,
                "Comparing se1/se2 with expected value {:?}",
                $ordering
            );
            assert_eq!(
                $se2.cmp(&$se1),
                inverse_ordering,
                "Comparing se2/se1 with expected value {:?}",
                inverse_ordering
            );
        };
    }

    fn make_simple(
        contour_id: usize,
        x: f64,
        y: f64,
        other_x: f64,
        other_y: f64,
        background: bool,
    ) -> (Entry, Entry) {
        let other = Rc::new(SweepEvent::new(
            Source {
                background,
                polygon: 0,
                command: 0,
            },
            contour_id,
            Point([other_x, other_y]),
            false,
            Weak::new(),
            true,
        ));
        let event = Rc::new(SweepEvent::new(
            Source {
                background,
                polygon: 0,
                command: 0,
            },
            contour_id,
            Point([x, y]),
            true,
            Rc::downgrade(&other),
            true,
        ));
        // Make sure test cases fulfill the invariant of left/right relationship.
        assert!(event.is_before(&other));

        (Entry(event), Entry(other))
    }

    #[test]
    fn not_collinear_shared_left_right_first() {
        let (se1, _other1) = make_simple(0, 0.0, 0.0, 1.0, 1.0, false);
        let (se2, _other2) = make_simple(0, 0.0, 0.0, 2.0, 3.0, false);

        let mut tree = Set::new();

        tree.insert(se1);
        tree.insert(se2);

        let min_other = tree.smallest().unwrap().other().unwrap();
        let max_other = tree.largest().unwrap().other().unwrap();

        assert_eq!(max_other.point, Point([2.0, 3.0]));
        assert_eq!(min_other.point, Point([1.0, 1.0]));
    }

    #[test]
    fn not_collinear_different_left_point_right_sort_y() {
        let (se1, _other1) = make_simple(0, 0.0, 1.0, 1.0, 1.0, false);
        let (se2, _other2) = make_simple(0, 0.0, 2.0, 2.0, 3.0, false);

        let mut tree = Set::new();

        tree.insert(se1);
        tree.insert(se2);

        let min_other = tree.smallest().unwrap().other().unwrap();
        let max_other = tree.largest().unwrap().other().unwrap();

        assert_eq!(min_other.point, Point([1.0, 1.0]));
        assert_eq!(max_other.point, Point([2.0, 3.0]));
    }

    #[test]
    fn not_collinear_order_in_sweep_line() {
        let (se1, _other1) = make_simple(0, 0.0, 1.0, 2.0, 1.0, false);
        let (se2, _other2) = make_simple(0, -1.0, 0.0, 2.0, 3.0, false);
        let (se3, _other3) = make_simple(0, 0.0, 1.0, 3.0, 4.0, false);
        let (se4, _other4) = make_simple(0, -1.0, 0.0, 3.0, 1.0, false);

        assert_eq!(se1.0.cmp(&*se2), Ordering::Less);
        assert!(!se2.is_below(se1.point));

        assert_ordering!(se1, &se2, Ordering::Less);

        assert_eq!(se3.0.cmp(&*se4), Ordering::Less);
        assert!(se4.is_below(se3.point));
    }

    #[test]
    fn not_collinear_first_point_is_below() {
        let (se2, _other2) = make_simple(0, 1.0, 1.0, 5.0, 1.0, false);
        let (se1, _other1) = make_simple(0, -1.0, 0.0, 2.0, 3.0, false);

        assert!(!se1.is_below(se2.point));
        assert_ordering!(se1, &se2, Ordering::Greater);
    }

    #[test]
    fn collinear_segments() {
        let (se1, _other1) = make_simple(0, 1.0, 1.0, 5.0, 1.0, true);
        let (se2, _other2) = make_simple(0, 2.0, 01.0, 3.0, 1.0, false);

        assert_ne!(se1.source.background, se2.source.background);
        assert_ordering!(se1, &se2, Ordering::Less);
    }

    #[test]
    fn collinear_shared_left_point() {
        {
            let (se1, _other2) = make_simple(1, 0.0, 1.0, 5.0, 1.0, false);
            let (se2, _other1) = make_simple(2, 0.0, 1.0, 3.0, 1.0, false);

            assert_eq!(se1.source.background, se2.source.background);
            assert_eq!(se1.point, se2.point);

            assert_ordering!(se1, &se2, Ordering::Less);
        }
        {
            let (se1, _other2) = make_simple(2, 0.0, 1.0, 5.0, 1.0, false);
            let (se2, _other1) = make_simple(1, 0.0, 1.0, 3.0, 1.0, false);

            assert_ordering!(se1, &se2, Ordering::Greater);
        }
    }

    #[test]
    fn collinear_same_polygon_different_left() {
        let (se1, _other2) = make_simple(0, 1.0, 1.0, 5.0, 1.0, true);
        let (se2, _other1) = make_simple(0, 2.0, 1.0, 3.0, 1.0, true);

        assert_eq!(se1.source.background, se2.source.background);
        assert_ne!(se1.point, se2.point);
        assert_ordering!(se1, &se2, Ordering::Less);
    }

    #[test]
    fn t_shaped_cases() {
        // shape:  /
        //        /\
        let (se1, _other1) = make_simple(0, 0.0, 0.0, 1.0, 1.0, true);
        let (se2, _other2) = make_simple(0, 0.5, 0.5, 1.0, 0.0, true);
        assert_ordering!(se1, &se2, Ordering::Greater);

        // shape: \/
        //         \
        let (se1, _other1) = make_simple(0, 0.0, 1.0, 1.0, 0.0, true);
        let (se2, _other2) = make_simple(0, 0.5, 0.5, 1.0, 1.0, true);
        assert_ordering!(se1, &se2, Ordering::Less);

        // shape: T
        let (se1, _other1) = make_simple(0, 0.0, 1.0, 1.0, 1.0, true);
        let (se2, _other2) = make_simple(0, 0.5, 0.0, 0.5, 1.0, true);
        assert_ordering!(se1, &se2, Ordering::Greater);

        // shape: T upside down
        let (se1, _other1) = make_simple(0, 0.0, 0.0, 1.0, 0.0, true);
        let (se2, _other2) = make_simple(0, 0.5, 0.0, 0.5, 1.0, true);
        assert_ordering!(se1, &se2, Ordering::Less);
    }

    #[test]
    fn vertical_segment() {
        // vertical reference segment at x = 0, expanding from y = -1 to +1.
        let (se1, _other1) = make_simple(0, 0.0, -1.0, 0.0, 1.0, true);

        // "above" cases
        let (se2, _other2) = make_simple(0, -1.0, 1.0, 0.0, 1.0, true);
        assert_ordering!(se1, &se2, Ordering::Less);
        let (se2, _other2) = make_simple(0, 0.0, 1.0, 1.0, 1.0, true);
        assert_ordering!(se1, &se2, Ordering::Less);
        let (se2, _other2) = make_simple(0, -1.0, 2.0, 0.0, 2.0, true);
        assert_ordering!(se1, &se2, Ordering::Less);
        let (se2, _other2) = make_simple(0, 0.0, 2.0, 1.0, 2.0, true);
        assert_ordering!(se1, &se2, Ordering::Less);
        let (se2, _other2) = make_simple(0, 0.0, 1.0, 0.0, 2.0, true);
        assert_ordering!(se1, &se2, Ordering::Less);

        // "below" cases
        let (se2, _other2) = make_simple(0, -1.0, -1.0, 0.0, -1.0, true);
        assert_ordering!(se1, &se2, Ordering::Greater);
        let (se2, _other2) = make_simple(0, 0.0, -1.0, 1.0, -1.0, true);
        assert_ordering!(se1, &se2, Ordering::Greater);
        let (se2, _other2) = make_simple(0, -1.0, -2.0, 0.0, -2.0, true);
        assert_ordering!(se1, &se2, Ordering::Greater);
        let (se2, _other2) = make_simple(0, 0.0, -2.0, 1.0, -2.0, true);
        assert_ordering!(se1, &se2, Ordering::Greater);
        let (se2, _other2) = make_simple(0, 0.0, -2.0, 0.0, -1.0, true);
        assert_ordering!(se1, &se2, Ordering::Greater);

        // overlaps
        let (se2, _other2) = make_simple(0, 0.0, -0.5, 0.0, 0.5, true);
        assert_ordering!(se1, &se2, Ordering::Less);
        // When left endpoints are identical, the ordering is no longer anti-symmetric.
        // TODO: Decide if this is a problem.
        // let (se2, _other2) = make_simple(0, 0.0, -1.0, 0.0, 0.0, true);
        // assert_ordering!(se1, se2, Ordering::Less); // fails because of its not anti-symmetric.
    }
}
