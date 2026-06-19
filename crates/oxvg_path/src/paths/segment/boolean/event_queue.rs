//! Implementation of the event-queue phase of the Martinez-Rueda algorithm.
use std::{
    collections::BinaryHeap,
    rc::{Rc, Weak},
};

use super::{
    splay::{Entry, Set},
    sweep_event::{EdgeType, ResultTransition, Source, SweepEvent},
    Operation,
};
use crate::{
    geometry::{Intersection, Line, Point},
    paths::events,
};

/// Creates a left-to-right queue of events (i.e. polygon edges) of the resulting
/// polygons.
///
/// This is done in two phases
/// 1. Using `EventQueue::fill` to construct the queue of events
/// 2. Using `EventQueue::subdivide` to pull events, creating new events at
///    intersections via subdivision.
pub struct EventQueue {
    /// Ordered heap of events.
    pub heap: BinaryHeap<Rc<SweepEvent>>,
    /// Background bounding-box.
    bbbox: (Point, Point),
    /// Foreground bounding-box.
    fbbox: (Point, Point),
    /// The boolean operation.
    operation: Operation,
}

impl EventQueue {
    /// Creates an event queue from two polygons.
    ///
    /// Each edge of the polygons are converted to a left/right `SweepEevent` pair.
    pub fn fill(
        background: &events::Path,
        foreground: &events::Path,
        operation: Operation,
    ) -> Self {
        let mut contour_id = 0;

        let bbox = (Point::INFINITY, Point::NEG_INFINITY);
        let mut result = Self {
            heap: BinaryHeap::new(),
            bbbox: bbox,
            fbbox: bbox,
            operation,
        };

        // 1. Gather background events and add them to the queue
        for (i, polygon) in background.0.iter().enumerate() {
            contour_id += 1;
            result.process_polygon(
                &polygon.data,
                Source {
                    background: true,
                    polygon: i,
                    command: 0,
                },
                contour_id,
                true,
            );
            for interior in &polygon.interiors {
                result.process_polygon(
                    interior,
                    Source {
                        background: true,
                        polygon: i,
                        command: 0,
                    },
                    contour_id,
                    false,
                );
            }
        }

        // 1. Gather foreground events an add them to the queue
        for (i, polygon) in foreground.0.iter().enumerate() {
            let exterior = operation != Operation::Difference;
            if exterior {
                contour_id += 1;
            }
            result.process_polygon(
                &polygon.data,
                Source {
                    background: false,
                    polygon: i,
                    command: 0,
                },
                contour_id,
                exterior,
            );
            for interior in &polygon.interiors {
                result.process_polygon(
                    interior,
                    Source {
                        background: false,
                        polygon: i,
                        command: 0,
                    },
                    contour_id,
                    false,
                );
            }
        }
        result
    }

    /// Returns true when the two shapes trivially do not overlap each other.
    pub fn is_trivial(&self) -> bool {
        self.bbbox.0.x() > self.fbbox.1.x()
            || self.fbbox.0.x() > self.bbbox.1.x()
            || self.bbbox.0.y() > self.fbbox.1.y()
            || self.fbbox.0.y() > self.bbbox.1.y()
    }

    /// Iterate through sorted events. Intersecting events will be subdivided.
    pub fn subdivide(&mut self) -> Vec<Rc<SweepEvent>> {
        let mut sweep_line = Set::new();
        let mut sorted_events: Vec<Rc<SweepEvent>> = vec![];
        let rightbound = self.bbbox.1.x().min(self.fbbox.1.x());

        while let Some(event) = self.heap.pop() {
            sorted_events.push(Rc::clone(&event));

            // For such operations, halts once the events no longer overlap
            if self.operation == Operation::Intersection && event.point.x() > rightbound
                || self.operation == Operation::Difference && event.point.x() > self.bbbox.1.x()
            {
                break;
            }

            // For the event, either inserts it or discards it from the result
            let entry = Entry(event.clone());
            if event.left() {
                sweep_line.insert(entry.clone());

                // 1. Get adjacent events and set transitions
                let maybe_prev = sweep_line.find_upper_bound_rev(&entry).cloned();
                let maybe_next = sweep_line.find_upper_bound(&entry).cloned();

                compute_fields(&event, maybe_prev.as_deref(), self.operation);

                if let Some(next) = maybe_next {
                    // 2. T-Intersecting `event -> next` transition is set
                    if self.possible_intersection(&event, &next) == 2 {
                        // Recompute fields for current segment and the one above (in bottom to top order)
                        compute_fields(&event, maybe_prev.as_deref(), self.operation);
                        compute_fields(&next, Some(&event), self.operation);
                    }
                }

                if let Some(prev) = maybe_prev {
                    // 3. T-intersecting `prev -> event` transition is set
                    if self.possible_intersection(&prev, &event) == 2 {
                        let maybe_prev_prev = sweep_line.find_upper_bound_rev(&prev);
                        // Recompute fields for current segment and the one below (in bottom to top order)
                        compute_fields(&prev, maybe_prev_prev.map(|e| &e.0), self.operation);
                        compute_fields(&event, Some(&prev), self.operation);
                    }
                }
            } else if let Some(other_event) = event.other() {
                // Replace this event with subdivision and handle it in next iteration
                let entry = Entry(other_event.clone());
                if sweep_line.contains(&entry) {
                    let maybe_prev = sweep_line.find_upper_bound_rev(&entry).cloned();
                    let maybe_next = sweep_line.find_upper_bound(&entry).cloned();

                    if let (Some(prev), Some(next)) = (maybe_prev, maybe_next) {
                        self.possible_intersection(&prev, &next);
                    }

                    sweep_line.remove(&entry);
                }
            }
        }

        sorted_events
    }

    /// Convert a polygon ring into events and push to the heap.
    fn process_polygon(
        &mut self,
        data: &[events::Data],
        mut source: Source,
        contour_id: usize,
        is_exterior: bool,
    ) {
        let bbox = if source.background {
            &mut self.bbbox
        } else {
            &mut self.fbbox
        };
        for (command, data) in data.iter().enumerate() {
            source.command = command;
            data.for_each(|line| {
                if line.start() == line.end() {
                    return;
                }

                let e1 = SweepEvent::new(
                    source.clone(),
                    contour_id,
                    *line.start(),
                    false,
                    Weak::new(),
                    is_exterior,
                );
                let e2 = SweepEvent::new(
                    source.clone(),
                    contour_id,
                    *line.end(),
                    false,
                    Rc::downgrade(&e1),
                    is_exterior,
                );
                *e1.other_mut() = Rc::downgrade(&e2);

                if e1 < e2 {
                    *e2.left_mut() = true;
                } else {
                    *e1.left_mut() = true;
                }

                *bbox.0.x_mut() = bbox.0.x().min(line.start().x());
                *bbox.0.y_mut() = bbox.0.y().min(line.start().y());
                *bbox.1.x_mut() = bbox.1.x().max(line.start().x());
                *bbox.1.y_mut() = bbox.1.y().max(line.start().y());

                self.heap.push(e1);
                self.heap.push(e2);
            });
        }
    }

    /// Return the type of event intersections
    ///
    /// 0. No intersection, or same source
    /// 1. Basic intersection
    /// 2. Coincidental left intersection
    /// 3. Partial/full parallel intersection
    fn possible_intersection(&mut self, a1: &Rc<SweepEvent>, b1: &Rc<SweepEvent>) -> u8 {
        let (Some(a2), Some(b2)) = (a1.other(), b1.other()) else {
            return 0;
        };

        match Line([a1.point, a2.point]).intersection(&Line([b1.point, b2.point])) {
            Intersection::None => 0,
            Intersection::Intersection(p) => {
                if a1.point == b1.point || a2.point == b2.point {
                    0
                } else {
                    if a1.point != p && a2.point != p {
                        self.divide_segment(a1, p);
                    }
                    if b1.point != p && b2.point != p {
                        self.divide_segment(b1, p);
                    }
                    1
                }
            }
            Intersection::Parallel(_) => {
                if a1.source.background == b1.source.background {
                    0
                } else {
                    let mut events = vec![];
                    let mut left_coincide = false;
                    let mut right_coincide = false;

                    if a1.point == b1.point {
                        left_coincide = true;
                    } else if a1 < b1 {
                        events.push((b1.clone(), b2.clone()));
                        events.push((a1.clone(), a2.clone()));
                    } else {
                        events.push((a1.clone(), a2.clone()));
                        events.push((b1.clone(), b2.clone()));
                    }

                    if a2.point == b2.point {
                        right_coincide = true;
                    } else if a2 < b2 {
                        events.push((b2, b1.clone()));
                        events.push((a2, a1.clone()));
                    } else {
                        events.push((a2, a1.clone()));
                        events.push((b2, b1.clone()));
                    }

                    if left_coincide {
                        *b1.edge_type_mut() = EdgeType::NonContributing;
                        if a1.in_out() == b1.in_out() {
                            *a1.edge_type_mut() = EdgeType::SameTransition;
                        } else {
                            *a1.edge_type_mut() = EdgeType::DifferentTransition;
                        }

                        if left_coincide && !right_coincide {
                            self.divide_segment(&events[1].1, events[0].0.point);
                        }
                        2
                    } else if right_coincide {
                        self.divide_segment(&events[0].0, events[1].0.point);
                        3
                    } else if !Rc::ptr_eq(&events[0].0, &events[3].1) {
                        self.divide_segment(&events[0].0, events[1].0.point);
                        self.divide_segment(&events[1].0, events[2].0.point);
                        3
                    } else {
                        self.divide_segment(&events[0].0, events[1].0.point);
                        self.divide_segment(&events[3].0.other().unwrap(), events[2].0.point);
                        3
                    }
                }
            }
        }
    }

    /// Subdivide the event at the intersection point and add the resulting events
    /// to the heap.
    pub fn divide_segment(&mut self, left: &Rc<SweepEvent>, mut intersection: Point) {
        debug_assert!(left.left());

        let Some(right) = left.other() else {
            return;
        };

        if intersection.x() == left.point.x() && intersection.y() < left.point.y() {
            *intersection.x_mut() = intersection.x().next_up();
        }

        let r = SweepEvent::new(
            left.source.clone(),
            left.contour_id,
            intersection,
            false,
            Rc::downgrade(left),
            true,
        );
        let l = SweepEvent::new(
            left.source.clone(),
            left.contour_id,
            intersection,
            true,
            Rc::downgrade(&right),
            true,
        );

        debug_assert!(left.is_before(&r));
        if !l.is_before(&right) {
            *right.left_mut() = true;
            *l.left_mut() = false;
        }

        *left.other_mut() = Rc::downgrade(&r);
        *right.other_mut() = Rc::downgrade(&l);

        self.heap.push(l);
        self.heap.push(r);
    }
}

/// Set the `in_out`, `other_in_out`, and `prev_in_result` fields of the event.
fn compute_fields(
    event: &Rc<SweepEvent>,
    maybe_prev: Option<&Rc<SweepEvent>>,
    operation: Operation,
) {
    if let Some(prev) = maybe_prev {
        if event.source.background == prev.source.background {
            event.set_in_out(!prev.in_out(), prev.other_in_out());
        } else if prev.is_vertical() {
            event.set_in_out(!prev.other_in_out(), !prev.in_out());
        } else {
            event.set_in_out(!prev.other_in_out(), prev.in_out());
        }

        if prev.is_in_result() && !prev.is_vertical() {
            *event.prev_in_result_mut() = Rc::downgrade(prev);
        } else if let Some(prev_of_prev) = prev.prev_in_result().upgrade() {
            *event.prev_in_result_mut() = Rc::downgrade(&prev_of_prev);
        } else {
            *event.prev_in_result_mut() = Weak::new();
        }
    } else {
        event.set_in_out(false, true);
        *event.prev_in_result_mut() = Weak::new();
    }

    let in_result = in_result(event, operation);
    let result_transition = if in_result {
        determine_result_transition(event, operation)
    } else {
        ResultTransition::None
    };
    *event.result_transition_mut() = result_transition;
}

/// Determin whether an event contributes to the result based on the boolean operation.
fn in_result(event: &SweepEvent, operation: Operation) -> bool {
    match *event.edge_type() {
        EdgeType::Normal => match operation {
            Operation::Intersection => !event.other_in_out(),
            Operation::Union => event.other_in_out(),
            Operation::Difference => {
                (event.source.background && event.other_in_out())
                    || (!event.source.background && !event.other_in_out())
            }
            Operation::Xor => true,
        },
        EdgeType::SameTransition => matches!(operation, Operation::Intersection | Operation::Union),
        EdgeType::DifferentTransition => matches!(operation, Operation::Difference),
        EdgeType::NonContributing => false,
    }
}

/// For a result, return whether the transition is from `in -> out` or `out -> in`.
fn determine_result_transition(event: &SweepEvent, operation: Operation) -> ResultTransition {
    let this_in = !event.in_out();
    let that_in = !event.other_in_out();
    let is_in = match operation {
        Operation::Intersection => this_in && that_in,
        Operation::Union => this_in || that_in,
        Operation::Xor => this_in ^ that_in,
        Operation::Difference => {
            if event.source.background {
                this_in && !that_in
            } else {
                that_in && !this_in
            }
        }
    };
    if is_in {
        ResultTransition::OutIn
    } else {
        ResultTransition::InOut
    }
}

#[cfg(test)]
mod divide_segments {
    use super::*;
    use std::rc::{Rc, Weak};

    fn make_simple(
        x: f64,
        y: f64,
        other_x: f64,
        other_y: f64,
        background: bool,
    ) -> (Rc<SweepEvent>, Rc<SweepEvent>) {
        let other = SweepEvent::new(
            Source {
                background,
                polygon: 0,
                command: 0,
            },
            0,
            Point([other_x, other_y]),
            false,
            Weak::new(),
            true,
        );
        let event = SweepEvent::new(
            Source {
                background,
                polygon: 0,
                command: 0,
            },
            0,
            Point([x, y]),
            true,
            Rc::downgrade(&other),
            true,
        );

        (event, other)
    }

    #[test]
    fn divide_segments() {
        let (a1, a2) = make_simple(0.0, 0.0, 5.0, 5.0, true);
        let (b1, b2) = make_simple(0.0, 5.0, 5.0, 0.0, false);
        let mut queue = EventQueue {
            heap: BinaryHeap::new(),
            bbbox: (Point::ZERO, Point::ZERO),
            fbbox: (Point::ZERO, Point::ZERO),
            operation: Operation::Union,
        };

        queue.heap.push(a1.clone());
        queue.heap.push(b1.clone());

        let Intersection::Intersection(inter) =
            Line([a1.point, a2.point]).intersection(&Line([b1.point, b2.point]))
        else {
            panic!("Not a point intersection");
        };

        queue.divide_segment(&a1, inter);
        queue.divide_segment(&b1, inter);

        assert_eq!(queue.heap.len(), 6);
    }
}

#[cfg(test)]
mod fill_queue {
    use super::*;
    use std::{
        cmp::Ordering,
        rc::{Rc, Weak},
    };

    fn make_simple(x: f64, y: f64, background: bool) -> Rc<SweepEvent> {
        SweepEvent::new(
            Source {
                background,
                polygon: 0,
                command: 0,
            },
            0,
            Point([x, y]),
            false,
            Weak::new(),
            true,
        )
    }

    fn check_order_in_queue(first: &Rc<SweepEvent>, second: &Rc<SweepEvent>) {
        let mut queue: BinaryHeap<Rc<SweepEvent>> = BinaryHeap::new();

        assert_eq!(first.cmp(second), Ordering::Greater);
        assert_eq!(second.cmp(first), Ordering::Less);
        {
            queue.push(first.clone());
            queue.push(second.clone());

            let p1 = queue.pop().unwrap();
            let p2 = queue.pop().unwrap();

            assert!(Rc::ptr_eq(first, &p1));
            assert!(Rc::ptr_eq(second, &p2));
        }
        {
            queue.push(second.clone());
            queue.push(first.clone());

            let p1 = queue.pop().unwrap();
            let p2 = queue.pop().unwrap();

            assert!(Rc::ptr_eq(first, &p1));
            assert!(Rc::ptr_eq(second, &p2));
        }
    }

    #[test]
    fn test_least_by_x() {
        check_order_in_queue(&make_simple(0.0, 0.0, false), &make_simple(0.5, 0.5, false));
    }

    #[test]
    fn test_least_by_y() {
        check_order_in_queue(&make_simple(0.0, 0.0, false), &make_simple(0.0, 0.5, false));
    }

    #[test]
    fn test_least_left() {
        let e1 = make_simple(0.0, 0.0, false);
        *e1.left_mut() = true;
        let e2 = make_simple(0.0, 0.0, false);
        *e2.left_mut() = false;

        check_order_in_queue(&e2, &e1);
    }

    #[test]
    fn test_shared_edge_not_colinear() {
        let other_e1 = make_simple(1.0, 1.0, false);
        let e1 = make_simple(0.0, 0.0, false);
        *e1.other_mut() = Rc::downgrade(&other_e1);
        *e1.left_mut() = true;
        let other_e2 = make_simple(2.0, 3.0, false);
        let e2 = make_simple(0.0, 0.0, false);
        *e2.other_mut() = Rc::downgrade(&other_e2);
        *e2.left_mut() = true;

        check_order_in_queue(&e1, &e2);
    }

    #[test]
    fn test_collinear_edges() {
        let other_e1 = make_simple(1.0, 1.0, true);
        let e1 = make_simple(0.0, 0.0, true);
        *e1.other_mut() = Rc::downgrade(&other_e1);
        *e1.left_mut() = true;
        let other_e2 = make_simple(2.0, 2.0, false);
        let e2 = make_simple(0.0, 0.0, false);
        *e2.other_mut() = Rc::downgrade(&other_e2);
        *e2.left_mut() = true;

        check_order_in_queue(&e1, &e2);
    }
}
