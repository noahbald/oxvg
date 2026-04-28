use std::{
    cell::{Ref, RefCell, RefMut},
    cmp::Ordering,
    rc::{Rc, Weak},
};

use crate::{
    geometry::{Line, Point},
    paths::segment::boolean::utils::{less_if, signed_area},
};

#[derive(Debug)]
pub enum EdgeType {
    Normal,
    NonContributing,
    SameTransition,
    DifferentTransition,
}

#[derive(PartialEq, Debug)]
pub enum ResultTransition {
    None,
    InOut,
    OutIn,
}

#[derive(Debug)]
pub struct Mutable {
    pub left: bool,
    pub other: Weak<SweepEvent>,
    pub in_out: bool,
    pub other_in_out: bool,
    pub result_transition: ResultTransition,
    pub prev_in_result: Weak<SweepEvent>,
    pub edge_type: EdgeType,
    pub other_pos: usize,
    pub output_contour_id: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Source {
    pub background: bool,
    pub polygon: usize,
    pub command: usize,
}

#[derive(Debug)]
pub struct SweepEvent {
    pub mutable: RefCell<Mutable>,
    pub source: Source,
    pub contour_id: usize,
    pub point: Point,
    #[allow(dead_code)]
    is_exterior: bool,
}

impl SweepEvent {
    pub fn new(
        source: Source,
        contour_id: usize,
        point: Point,
        left: bool,
        other: Weak<Self>,
        is_exterior: bool,
    ) -> Self {
        Self {
            mutable: RefCell::new(Mutable {
                left,
                other,
                in_out: false,
                other_in_out: false,
                result_transition: ResultTransition::None,
                prev_in_result: Weak::new(),
                edge_type: EdgeType::Normal,
                other_pos: 0,
                output_contour_id: 0,
            }),
            source,
            contour_id,
            point,
            is_exterior,
        }
    }

    pub fn left(&self) -> bool {
        self.mutable.borrow().left
    }

    pub fn left_mut(&'_ self) -> RefMut<'_, bool> {
        RefMut::map(self.mutable.borrow_mut(), |m| &mut m.left)
    }

    pub fn other(&self) -> Option<Rc<Self>> {
        self.mutable.borrow().other.upgrade()
    }

    pub fn other_mut(&'_ self) -> RefMut<'_, Weak<Self>> {
        RefMut::map(self.mutable.borrow_mut(), |m| &mut m.other)
    }

    pub fn in_out(&self) -> bool {
        self.mutable.borrow().in_out
    }

    pub fn other_in_out(&self) -> bool {
        self.mutable.borrow().other_in_out
    }

    pub fn set_in_out(&self, in_out: bool, other_in_out: bool) {
        let mut m = self.mutable.borrow_mut();
        m.in_out = in_out;
        m.other_in_out = other_in_out;
    }

    pub fn is_before(&self, other: &Self) -> bool {
        self > other
    }

    pub fn is_vertical(&self) -> bool {
        match self.other() {
            Some(other) => Line([self.point, other.point]).is_vertical(),
            None => false,
        }
    }

    pub fn result_transition(&'_ self) -> Ref<'_, ResultTransition> {
        Ref::map(self.mutable.borrow(), |m| &m.result_transition)
    }

    pub fn result_transition_mut(&'_ self) -> RefMut<'_, ResultTransition> {
        RefMut::map(self.mutable.borrow_mut(), |m| &mut m.result_transition)
    }

    pub fn is_in_result(&self) -> bool {
        *self.result_transition() != ResultTransition::None
    }

    pub fn prev_in_result(&'_ self) -> Ref<'_, Weak<SweepEvent>> {
        Ref::map(self.mutable.borrow(), |m| &m.prev_in_result)
    }

    pub fn prev_in_result_mut(&'_ self) -> RefMut<'_, Weak<SweepEvent>> {
        RefMut::map(self.mutable.borrow_mut(), |m| &mut m.prev_in_result)
    }

    pub fn edge_type(&'_ self) -> Ref<'_, EdgeType> {
        Ref::map(self.mutable.borrow(), |m| &m.edge_type)
    }

    pub fn edge_type_mut(&'_ self) -> RefMut<'_, EdgeType> {
        RefMut::map(self.mutable.borrow_mut(), |m| &mut m.edge_type)
    }

    pub fn other_pos(&self) -> usize {
        self.mutable.borrow().other_pos
    }

    pub fn other_pos_mut(&'_ self) -> RefMut<'_, usize> {
        RefMut::map(self.mutable.borrow_mut(), |m| &mut m.other_pos)
    }

    pub fn output_contour_id(&self) -> usize {
        self.mutable.borrow().output_contour_id
    }

    pub fn output_contour_id_mut(&'_ self) -> RefMut<'_, usize> {
        RefMut::map(self.mutable.borrow_mut(), |m| &mut m.output_contour_id)
    }
}

impl PartialEq for SweepEvent {
    fn eq(&self, other: &Self) -> bool {
        self.contour_id == other.contour_id
            && self.left() == other.left()
            && self.point == other.point
            && self.source.background == other.source.background
    }
}
impl Eq for SweepEvent {}

impl PartialOrd for SweepEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SweepEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.point.x() > other.point.x() {
            return Ordering::Less;
        } else if self.point.x() < other.point.x() {
            return Ordering::Greater;
        } else if self.point.y() > other.point.y() {
            return Ordering::Less;
        } else if self.point.y() < other.point.y() {
            return Ordering::Greater;
        } else if self.left() != other.left() {
            return less_if(self.left());
        } else if let (Some(other), Some(other2)) = (self.other(), other.other()) {
            if signed_area(self.point, other.point, other2.point) != 0.0 {
                return less_if(!self.is_below(other2.point));
            }
        }
        less_if(!self.source.background && other.source.background)
    }
}

impl SweepEvent {
    pub fn is_below(&self, p: Point) -> bool {
        if let Some(other) = self.other() {
            if self.left() {
                signed_area(self.point, other.point, p) > 0.0
            } else {
                signed_area(other.point, self.point, p) > 0.0
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub fn se_pair(
        contour_id: usize,
        x: f64,
        y: f64,
        other_x: f64,
        other_y: f64,
        background: bool,
    ) -> (Rc<SweepEvent>, Rc<SweepEvent>) {
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

        (event, other)
    }

    #[test]
    pub fn test_is_below() {
        let other_s1 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point::UNIT,
            false,
            Weak::new(),
            true,
        ));
        let s1 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point::ZERO,
            true,
            Rc::downgrade(&other_s1),
            true,
        ));
        let s2 = SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point::ZERO,
            false,
            Rc::downgrade(&s1),
            true,
        );

        assert!(s1.is_below(Point([0.0, 1.0])));
        assert!(s1.is_below(Point([1.0, 2.0])));
        assert!(!s1.is_below(Point([0.0, 0.0])));
        assert!(!s1.is_below(Point([5.0, -1.0])));

        assert!(!s2.is_below(Point([0.0, 1.0])));
        assert!(!s2.is_below(Point([1.0, 2.0])));
        assert!(!s2.is_below(Point([0.0, 0.0])));
        assert!(!s2.is_below(Point([5.0, -1.0])));
    }

    #[test]
    pub fn test_is_above() {
        let other_s1 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point([1.0, 1.0]),
            false,
            Weak::new(),
            true,
        ));
        let s1 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point([0.0, 0.0]),
            true,
            Rc::downgrade(&other_s1),
            true,
        ));
        let s2 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point([0.0, 1.0]),
            false,
            Rc::downgrade(&s1),
            true,
        ));

        assert!(s1.is_below(Point([0.0, 1.0])));
        assert!(s1.is_below(Point([1.0, 2.0])));
        assert!(!s1.is_below(Point([0.0, 0.0])));
        assert!(!s1.is_below(Point([5.0, -1.0])));

        assert!(!s2.is_below(Point([0.0, 1.0])));
        assert!(!s2.is_below(Point([1.0, 2.0])));
        assert!(!s2.is_below(Point([0.0, 0.0])));
        assert!(!s2.is_below(Point([5.0, -1.0])));
    }

    #[test]
    pub fn test_is_vertical() {
        let other_s1 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point([0.0, 1.0]),
            false,
            Weak::new(),
            true,
        ));
        let s1 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point([0.0, 0.0]),
            true,
            Rc::downgrade(&other_s1),
            true,
        ));
        let other_s2 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point([0.0001, 1.0]),
            false,
            Weak::new(),
            true,
        ));
        let s2 = Rc::new(SweepEvent::new(
            Source {
                background: false,
                polygon: 0,
                command: 0,
            },
            0,
            Point([0.0, 0.0]),
            true,
            Rc::downgrade(&other_s2),
            true,
        ));

        assert!(s1.is_vertical());
        assert!(!s2.is_vertical());
    }

    #[rustfmt::skip]
    #[test]
    fn test_order_star_pattern() {
        // This test verifies the assumption underlying the `precompute_iteration_order` logic:
        // Events with an identical points must be ordered:
        // - R events before L events
        // - R events in clockwise order
        // - L events in counter-clockwise order
        let id = 0;
        let z = 0.;

        // Group 'a' which have their right event at (0, 0), clockwise
        let (_av_l, av_r) = se_pair(id,  0., -1., z, z, true);   // vertical comes first
        let (_a1_l, a1_r) = se_pair(id, -2., -6., z, z, true);
        let (_a2_l, a2_r) = se_pair(id, -1., -2., z, z, true);
        let (_a3_l, a3_r) = se_pair(id, -1., -1., z, z, true);
        let (_a4_l, a4_r) = se_pair(id, -2., -1., z, z, true);
        let (_a5_l, a5_r) = se_pair(id, -2.,  1., z, z, true);
        let (_a6_l, a6_r) = se_pair(id, -1.,  1., z, z, true);
        let (_a7_l, a7_r) = se_pair(id, -1.,  2., z, z, true);
        let (_a8_l, a8_r) = se_pair(id, -2.,  6., z, z, true);

        // Group 'b' which have their left event at (0, 0), counter clockwise
        let (b1_l, _b1_r) = se_pair(id, z, z, 2., -6., true);
        let (b2_l, _b2_r) = se_pair(id, z, z, 1., -2., true);
        let (b3_l, _b3_r) = se_pair(id, z, z, 1., -1., true);
        let (b4_l, _b4_r) = se_pair(id, z, z, 2., -1., true);
        let (b5_l, _b5_r) = se_pair(id, z, z, 2.,  1., true);
        let (b6_l, _b6_r) = se_pair(id, z, z, 1.,  1., true);
        let (b7_l, _b7_r) = se_pair(id, z, z, 1.,  2., true);
        let (b8_l, _b8_r) = se_pair(id, z, z, 2.,  6., true);
        let (bv_l, _bv_r) = se_pair(id, z, z, 0.,  1., true);    // vertical comes last

        let events_expected_order = [
            av_r, a1_r, a2_r, a3_r, a4_r, a5_r, a6_r, a7_r, a8_r,
            b1_l, b2_l, b3_l, b4_l, b5_l, b6_l, b7_l, b8_l, bv_l,
        ];

        for i in 0 .. events_expected_order.len() - 1 {
            for j in i + 1 .. events_expected_order.len() {
                assert!(events_expected_order[i].is_before(&events_expected_order[j]));
            }
        }

    }
}
