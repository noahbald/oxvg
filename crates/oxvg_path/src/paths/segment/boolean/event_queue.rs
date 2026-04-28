use std::{
    collections::BinaryHeap,
    rc::{Rc, Weak},
};

use crate::{
    geometry::{Intersection, Line, Point},
    paths::{
        events,
        segment::boolean::{
            splay::{Entry, Set},
            sweep_event::{EdgeType, ResultTransition, Source, SweepEvent},
            Operation,
        },
    },
};

#[derive(Debug)]
pub struct EventQueue {
    heap: BinaryHeap<Rc<SweepEvent>>,
    bbbox: (Point, Point),
    fbbox: (Point, Point),
    operation: Operation,
}

impl EventQueue {
    pub fn fill(
        background: &events::Path,
        foreground: &events::Path,
        operation: Operation,
    ) -> Self {
        let mut contour_id = 0;

        let mut result = Self {
            heap: BinaryHeap::new(),
            bbbox: (Point::INFINITY, Point::NEG_INFINITY),
            fbbox: (Point::INFINITY, Point::NEG_INFINITY),
            operation,
        };
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
            // TODO: holes when fill-rule is evenodd
            // for interior in polygon.interiors() {
            //     process_polygon(interior, true, contour_id, &mut event_queue, self.bbbox, false);
            // }
        }

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
            )
            // TODO: holes when fill-rule is evenodd
            // for interior in polygon.interiors() {
            //     process_polygon(interior, false, contour_id, &mut event_queue, cbbox, false);
            // }
        }
        result
    }

    pub fn is_trivial(&self) -> bool {
        let bbbox = self.bbbox;
        let fbbox = self.fbbox;
        bbbox.0.x() > fbbox.1.x()
            || fbbox.0.x() > bbbox.1.x()
            || bbbox.0.y() > fbbox.1.y()
            || fbbox.0.y() > bbbox.1.y()
    }

    pub fn subdivide(&mut self) -> Vec<Rc<SweepEvent>> {
        let mut sweep_line = Set::new();
        let mut sorted_events: Vec<Rc<SweepEvent>> = vec![];
        let rightbound = self.bbbox.1.x().min(self.fbbox.1.x());

        while let Some(event) = self.heap.pop() {
            sorted_events.push(Rc::clone(&event));

            if self.operation == Operation::Intersection && event.point.x() > rightbound
                || self.operation == Operation::Difference && event.point.x() > self.bbbox.1.x()
            {
                break;
            }

            let entry = Entry(event.clone());
            if event.left() {
                sweep_line.insert(entry.clone());

                let maybe_prev = sweep_line.find_upper_bound_inv(&entry).cloned();
                let maybe_next = sweep_line.find_upper_bound(&entry).cloned();

                compute_fields(&event, maybe_prev.as_deref(), self.operation);

                if let Some(next) = maybe_next {
                    if self.possible_intersection(&event, &next) == 2 {
                        // Recompute fields for current segment and the one above (in bottom to top order)
                        compute_fields(&event, maybe_prev.as_deref(), self.operation);
                        compute_fields(&next, Some(&event), self.operation);
                    }
                }

                if let Some(prev) = maybe_prev {
                    if self.possible_intersection(&prev, &event) == 2 {
                        let maybe_prev_prev = sweep_line.find_upper_bound_inv(&prev);
                        // Recompute fields for current segment and the one below (in bottom to top order)
                        compute_fields(&prev, maybe_prev_prev.map(|e| &e.0), self.operation);
                        compute_fields(&event, Some(&prev), self.operation);
                    }
                }
            } else if let Some(other_event) = event.other() {
                let entry = Entry(other_event.clone());
                // This debug assert is only true, if we compare segments in the sweep line
                // based on identity (curently), and not by value (done previously).
                debug_assert!(
                    sweep_line.contains(&entry),
                    "Sweep line misses event to be removed"
                );
                if sweep_line.contains(&entry) {
                    let maybe_prev = sweep_line.find_upper_bound_inv(&entry).cloned();
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

                let e1 = Rc::new(SweepEvent::new(
                    source.clone(),
                    contour_id,
                    *line.start(),
                    false,
                    Weak::new(),
                    is_exterior,
                ));
                let e2 = Rc::new(SweepEvent::new(
                    source.clone(),
                    contour_id,
                    *line.end(),
                    false,
                    Rc::downgrade(&e1),
                    is_exterior,
                ));
                *e1.other_mut() = Rc::downgrade(&e2);

                if e1 < e2 {
                    *e2.left_mut() = true
                } else {
                    *e1.left_mut() = true
                }

                bbox.0 = Point([
                    bbox.0.x().min(line.start().x()),
                    bbox.0.y().min(line.start().y()),
                ]);
                bbox.1 = Point([
                    bbox.1.x().max(line.start().x()),
                    bbox.1.y().max(line.start().y()),
                ]);

                self.heap.push(e1);
                self.heap.push(e2);
            });
        }
    }

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
                        self.divide_segment(&b1, p);
                    }
                    1
                }
            }
            Intersection::Parallel(..) => {
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
                        events.push((b2.clone(), b1.clone()));
                        events.push((a2.clone(), a1.clone()));
                    } else {
                        events.push((a2.clone(), a1.clone()));
                        events.push((b2.clone(), b1.clone()));
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

    pub fn divide_segment(&mut self, left: &Rc<SweepEvent>, mut intersection: Point) {
        debug_assert!(left.left());

        let Some(right) = left.other() else {
            return;
        };

        if intersection.x() == left.point.x() && intersection.y() < left.point.y() {
            *intersection.x_mut() = intersection.x().next_up();
        }

        let r = Rc::new(SweepEvent::new(
            left.source.clone(),
            left.contour_id,
            intersection,
            false,
            Rc::downgrade(left),
            true,
        ));
        let l = Rc::new(SweepEvent::new(
            left.source.clone(),
            left.contour_id,
            intersection,
            true,
            Rc::downgrade(&right),
            true,
        ));

        debug_assert!(left.is_before(&right));
        if !left.is_before(&right) {
            *right.left_mut() = true;
            *l.left_mut() = false;
        }

        *left.other_mut() = Rc::downgrade(&r);
        *right.other_mut() = Rc::downgrade(&l);

        self.heap.push(l);
        self.heap.push(r);
    }
}

fn compute_fields(
    event: &Rc<SweepEvent>,
    maybe_prev: Option<&Rc<SweepEvent>>,
    operation: Operation,
) {
    if let Some(prev) = maybe_prev {
        if event.source.background == prev.source.background {
            event.set_in_out(!prev.in_out(), prev.other_in_out())
        } else if prev.is_vertical() {
            event.set_in_out(!prev.other_in_out(), !prev.in_out())
        } else {
            event.set_in_out(!prev.other_in_out(), prev.in_out())
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
    let result_transition = if !in_result {
        ResultTransition::None
    } else {
        determine_result_transition(event, operation)
    };
    *event.result_transition_mut() = result_transition;
}

fn in_result(event: &Rc<SweepEvent>, operation: Operation) -> bool {
    match &*event.edge_type() {
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

fn determine_result_transition(event: &Rc<SweepEvent>, operation: Operation) -> ResultTransition {
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
        let other = Rc::new(SweepEvent::new(
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
        ));
        let event = Rc::new(SweepEvent::new(
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
        ));

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

        let inter = match Line([a1.point, a2.point]).intersection(&Line([b1.point, b2.point])) {
            Intersection::Intersection(p) => p,
            _ => panic!("Not a point intersection"),
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
        Rc::new(SweepEvent::new(
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
        ))
    }

    fn check_order_in_queue(first: Rc<SweepEvent>, second: Rc<SweepEvent>) {
        let mut queue: BinaryHeap<Rc<SweepEvent>> = BinaryHeap::new();

        assert_eq!(first.cmp(&second), Ordering::Greater);
        assert_eq!(second.cmp(&first), Ordering::Less);
        {
            queue.push(first.clone());
            queue.push(second.clone());

            let p1 = queue.pop().unwrap();
            let p2 = queue.pop().unwrap();

            assert!(Rc::ptr_eq(&first, &p1));
            assert!(Rc::ptr_eq(&second, &p2));
        }
        {
            queue.push(second.clone());
            queue.push(first.clone());

            let p1 = queue.pop().unwrap();
            let p2 = queue.pop().unwrap();

            assert!(Rc::ptr_eq(&first, &p1));
            assert!(Rc::ptr_eq(&second, &p2));
        }
    }

    #[test]
    fn test_least_by_x() {
        check_order_in_queue(make_simple(0.0, 0.0, false), make_simple(0.5, 0.5, false))
    }

    #[test]
    fn test_least_by_y() {
        check_order_in_queue(make_simple(0.0, 0.0, false), make_simple(0.0, 0.5, false))
    }

    #[test]
    fn test_least_left() {
        let e1 = make_simple(0.0, 0.0, false);
        *e1.left_mut() = true;
        let e2 = make_simple(0.0, 0.0, false);
        *e2.left_mut() = false;

        check_order_in_queue(e2, e1)
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

        check_order_in_queue(e1, e2)
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

        check_order_in_queue(e1, e2)
    }
}
