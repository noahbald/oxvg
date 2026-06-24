//! Trace output contours from classified sweep events.
use std::{collections::HashSet, rc::Rc};

use super::sweep_event::{ResultTransition, Source, SweepEvent};
use crate::{
    geometry::Point,
    paths::{
        events,
        segment::{Data, Segment, ToleranceSquared},
    },
};

#[derive(Debug)]
/// A closed ring forming the output polygon.
pub struct Contour {
    /// Ordered edges of the ring.
    pub edges: Vec<(Point, Point, Source)>,
    /// Contours that make up the holes of this contour.
    pub holes: Vec<usize>,
    /// Index this contour is a hole of, `None` if an exterior contour.
    pub hole_of: Option<usize>,
    /// Nesting depth of contour.
    pub depth: usize,
}

impl Contour {
    fn new(hole_of: Option<usize>, depth: usize) -> Self {
        Self {
            edges: vec![],
            holes: vec![],
            hole_of,
            depth,
        }
    }

    /// Determines nesting context for a new contour starting at the given event.
    fn initialise_from_context(
        event: &Rc<SweepEvent>,
        contours: &mut [Contour],
        contour_id: usize,
    ) -> Self {
        if let Some(prev_in_result) = event.prev_in_result().upgrade() {
            // Determine placement of contour
            let lower_contour_id = prev_in_result.output_contour_id();
            if *prev_in_result.result_transition() == ResultTransition::OutIn {
                // Edge is start of out-in; start of a hole
                let lower_contour = &contours[lower_contour_id];
                if let Some(parent_contour_id) = lower_contour.hole_of {
                    // Edge is adjacent to another hole
                    contours[parent_contour_id].holes.push(contour_id);
                    let hole_of = Some(parent_contour_id);
                    let depth = contours[lower_contour_id].depth;
                    Contour::new(hole_of, depth)
                } else {
                    // Edge is within another hole
                    contours[lower_contour_id].holes.push(contour_id);
                    let hole_of = Some(lower_contour_id);
                    let depth = contours[lower_contour_id].depth + 1;
                    Contour::new(hole_of, depth)
                }
            } else {
                // Edge is adjacent to anther contour
                let depth = if lower_contour_id >= contours.len() {
                    panic!("invalid lower-contour id");
                } else {
                    contours[lower_contour_id].depth
                };
                Contour::new(None, depth)
            }
        } else {
            // This is the first contour.
            Contour::new(None, 0)
        }
    }

    /// Returns whether this contour is an outermost contour.
    pub fn is_exterior(&self) -> bool {
        self.hole_of.is_none()
    }

    /// Reconstruct the curved geometry for this contour.
    pub fn slice(
        &self,
        background: &events::Path,
        foreground: &events::Path,
        tolerance: &ToleranceSquared,
    ) -> Option<Segment> {
        if !self.is_exterior() {
            // TODO: support holes, ensure correct fill-rule
            return None;
        }
        let mut segment = Segment {
            start: self.edges[0].0,
            data: vec![],
            closed: true,
        };
        for (start, end, source_ref) in &self.edges {
            let source = if source_ref.background {
                background
            } else {
                foreground
            };
            let source = &source.0[source_ref.polygon];
            let source = &source.data[source_ref.command];

            // Reconstruct SVG lines, curves, and arc.
            segment.data.push(match source {
                events::Data::Line(_) => {
                    // Lines reconstructed as-is
                    Data::LineTo(*end)
                }
                events::Data::Curve(curve, p) => {
                    // Curve reconstructed by slicing and reversing as needed
                    let curve_start = p[0].start();
                    let curve_end = p.last().unwrap().end();
                    if *start == curve_start && curve_end == *end {
                        Data::CurveTo(*curve)
                    } else if *start == curve_end && curve_start == *end {
                        Data::CurveTo(curve.reverse(curve_start))
                    } else {
                        let t1 = if *start == p[0].start() {
                            0.0
                        } else if *start == curve_end {
                            1.0
                        } else {
                            curve.t_at(curve_start, *start, tolerance).unwrap()
                        };
                        let t2 = if *end == curve_end {
                            1.0
                        } else if *end == curve_start {
                            0.0
                        } else {
                            curve.t_at(curve_start, *end, tolerance).unwrap()
                        };
                        Data::CurveTo(if t1 <= t2 {
                            curve.clamp_t(curve_start, t1, t2)
                        } else {
                            curve.clamp_t(curve_start, t2, t1).reverse(*end)
                        })
                    }
                }
                events::Data::Arc(arc, p) => {
                    // Arcs reconstructed by slicing and reversing as needed
                    let arc_start = p[0].start();
                    let arc_end = p.last().unwrap().end();
                    if *start == arc_start && arc_end == *end {
                        Data::ArcTo(arc.clone())
                    } else if *start == arc_end && arc_start == *end {
                        // Edge runs opposite to arc's natural direction: full arc reversed
                        Data::ArcTo(arc.reverse().with_end_point_memo(*start))
                    } else {
                        let t1 = if *start == arc_start {
                            0.0
                        } else if *start == arc_end {
                            1.0
                        } else {
                            arc.t_at(*start, tolerance).unwrap()
                        };
                        let t2 = if *end == arc_end {
                            1.0
                        } else if *end == arc_start {
                            0.0
                        } else {
                            arc.t_at(*end, tolerance).unwrap()
                        };
                        Data::ArcTo(if t1 <= t2 {
                            arc.clamp_t(t1, t2)
                        } else {
                            arc.clamp_t(t2, t1).reverse()
                        })
                    }
                }
            });
        }

        Some(segment)
    }
}

/// Assemble events into contours.
pub fn connect_edges(sorted_events: Vec<Rc<SweepEvent>>) -> Vec<Contour> {
    // Filter events to in-result events
    let result_events = order_events(sorted_events);

    // Chain group of events together
    let iteration_map = precompute_iteration_order(&result_events);

    let mut contours: Vec<Contour> = vec![];
    let mut processed: HashSet<usize> = HashSet::new();

    // Trace each group in to contours
    for i in 0..result_events.len() {
        if processed.contains(&i) {
            // Event is part of an already processed group
            continue;
        }

        // Initialise this group
        let contour_id = contours.len();
        let mut contour =
            Contour::initialise_from_context(&result_events[i], &mut contours, contour_id);

        let mut pos = i;

        let initial = result_events[i].point;
        contour
            .edges
            .push((initial, initial, result_events[i].source.clone()));

        // Trace the group
        loop {
            // Add left and right event to `processed`
            processed.insert(pos);
            *result_events[pos].output_contour_id_mut() = contour_id;

            let left_point = result_events[pos].point;
            pos = result_events[pos].other_pos();

            processed.insert(pos);
            *result_events[pos].output_contour_id_mut() = contour_id;

            let right_point = result_events[pos].point;

            // Add left and right event to contour
            if contour.edges.last().map(|e| &e.2) == Some(&result_events[pos].source.clone()) {
                // Event continues source of previous event, so extend the event.
                // This is because many events may lie on a single arc/curve that will be reconstructed.
                contour.edges.last_mut().unwrap().1 = right_point;
            } else {
                // Event is start of a new source, so push the event.
                contour
                    .edges
                    .push((left_point, right_point, result_events[pos].source.clone()));
            }

            // get `pos` for next event
            let next_pos_opt = get_next_pos(pos, &processed, &iteration_map);
            match next_pos_opt {
                Some(npos) => pos = npos,
                None => break, // no next pos, end contour
            }

            // `pos` hasn't progressed/returned to start, so the contour has ended
            if pos == i {
                break;
            }
        }

        contours.push(contour);
    }
    contours
}

/// Filter events and set up `other_pos` links.
fn order_events(sorted_events: Vec<Rc<SweepEvent>>) -> Vec<Rc<SweepEvent>> {
    // Keeps only left/right events that are in results
    let mut result_events: Vec<_> = sorted_events
        .into_iter()
        .filter(|event| {
            (event.left() && event.is_in_result())
                || (!event.left() && event.other().is_some_and(|o| o.is_in_result()))
        })
        .collect();

    // Update `other_pos`
    result_events.sort_by(|a, b| b.cmp(a));
    for (pos, event) in result_events.iter().enumerate() {
        *event.other_pos_mut() = pos;
    }
    for event in &result_events {
        if event.left() {
            if let Some(other) = event.other() {
                let (a, b) = (event.other_pos(), other.other_pos());
                *event.other_pos_mut() = b;
                *other.other_pos_mut() = a;
            }
        }
    }
    result_events
}

/// Build an iteration map over events to control visit order.
///
/// Each group will be chained together.
/// Each group will be iterated from right to left.
pub fn precompute_iteration_order(data: &[Rc<SweepEvent>]) -> Vec<usize> {
    let mut map = vec![0; data.len()];

    let mut i = 0;
    while i < data.len() {
        let x = &data[i];

        let r_from = i;
        while i < data.len() && x.point == data[i].point && !data[i].left() {
            i += 1;
        }
        let r_to = i;

        let l_from = i;
        while i < data.len() && x.point == data[i].point {
            debug_assert!(data[i].left());
            i += 1;
        }
        let l_to = i;

        let has_r = r_to > r_from;
        let has_l = l_to > l_from;

        if has_r {
            let to = r_to - 1;
            for (j, item) in map.iter_mut().enumerate().take(to).skip(r_from) {
                *item = j + 1;
            }
            if has_l {
                map[to] = l_to - 1;
            } else {
                map[to] = r_from;
            }
        }
        if has_l {
            let to = l_to - 1;
            for (j, item) in map.iter_mut().enumerate().take(to + 1).skip(l_from + 1) {
                *item = j - 1;
            }
            if has_r {
                map[l_from] = r_from;
            } else {
                map[l_from] = to;
            }
        }
    }
    map
}

// Update `pos` to the next event that leads from the current pos.
//
// Returns `None` if this is the last pos.
fn get_next_pos(
    mut pos: usize,
    processed: &HashSet<usize>,
    iteration_map: &[usize],
) -> Option<usize> {
    let start_pos = pos;

    loop {
        pos = iteration_map[pos];
        if pos == start_pos {
            return None;
        } else if !processed.contains(&pos) {
            return Some(pos);
        }
    }
}
