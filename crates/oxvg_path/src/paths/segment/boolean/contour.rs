use std::{collections::HashSet, rc::Rc};

use super::sweep_event::{ResultTransition, Source, SweepEvent};
use crate::{
    geometry::Point,
    paths::{
        events,
        segment::{Data, Segment, ToleranceSquared},
    },
};

pub struct Contour {
    pub edges: Vec<(Point, Point, Source)>,
    pub holes: Vec<usize>,
    pub hole_of: Option<usize>,
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

    fn initialise_from_context(
        event: &Rc<SweepEvent>,
        contours: &mut [Contour],
        contour_id: usize,
    ) -> Self {
        if let Some(prev_in_result) = event.prev_in_result().upgrade() {
            let lower_contour_id = prev_in_result.output_contour_id();
            if *prev_in_result.result_transition() == ResultTransition::OutIn {
                let lower_contour = &contours[lower_contour_id];
                if let Some(parent_contour_id) = lower_contour.hole_of {
                    contours[parent_contour_id].holes.push(contour_id);
                    let hole_of = Some(parent_contour_id);
                    let depth = contours[lower_contour_id].depth;
                    Contour::new(hole_of, depth)
                } else {
                    contours[lower_contour_id].holes.push(contour_id);
                    let hole_of = Some(lower_contour_id);
                    let depth = contours[lower_contour_id].depth + 1;
                    Contour::new(hole_of, depth)
                }
            } else {
                let depth = if lower_contour_id >= contours.len() {
                    panic!("invalid lower-contour id");
                } else {
                    contours[lower_contour_id].depth
                };
                Contour::new(None, depth)
            }
        } else {
            Contour::new(None, 0)
        }
    }

    pub fn is_exterior(&self) -> bool {
        self.hole_of.is_none()
    }

    pub fn slice(
        &self,
        background: &events::Path,
        foreground: &events::Path,
        tolerance: &ToleranceSquared,
    ) -> Option<Segment> {
        if !self.is_exterior() {
            return None;
        }
        let mut segment = Segment {
            start: self.edges[0].0,
            data: vec![],
            closed: true,
        };
        dbg!(&self.edges);
        for (start, end, source_ref) in &self.edges {
            let source = if source_ref.background {
                background
            } else {
                foreground
            };
            let source = &source.0[source_ref.polygon];
            let source = &source.data[source_ref.command];

            match source {
                events::Data::Line(p) => {
                    dbg!(p);
                }
                events::Data::Curve(c, _) => {
                    dbg!(c);
                }
                events::Data::Arc(a, _) => {
                    dbg!(a);
                }
            }

            segment.data.push(match source {
                events::Data::Line(_) => Data::LineTo(*end),
                events::Data::Curve(curve, p) => {
                    let curve_start = *p[0].start();
                    let curve_end = *p.last().unwrap().end();
                    dbg!(start, curve_start, curve_end, end);
                    if *start == curve_start && curve_end == *end {
                        Data::CurveTo(*curve)
                    } else {
                        let t1 = if *start == *p[0].start() {
                            0.0
                        } else {
                            curve.t_at(curve_start, *start, tolerance).unwrap()
                        };
                        let t2 = if *end == curve_end {
                            1.0
                        } else {
                            curve.t_at(curve_start, *end, tolerance).unwrap()
                        };
                        Data::CurveTo(if dbg!(t1) <= dbg!(t2) {
                            curve.clamp_t(curve_start, t1, t2)
                        } else {
                            curve.clamp_t(curve_start, t2, t1).reverse(curve_start)
                        })
                    }
                }
                events::Data::Arc(arc, p) => {
                    let arc_start = *p[0].start();
                    let arc_end = *p.last().unwrap().end();
                    dbg!(start, arc_start, arc_end, end);
                    if *start == arc_start && arc_end == *end {
                        Data::ArcTo(*arc)
                    } else {
                        let t1 = if *start == arc_start {
                            0.0
                        } else {
                            arc.t_at(*start, tolerance).unwrap()
                        };
                        let t2 = if *end == arc_end {
                            1.0
                        } else {
                            arc.t_at(*end, tolerance).unwrap()
                        };
                        Data::ArcTo(if dbg!(t1) <= dbg!(t2) {
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

pub fn connect_edges(sorted_events: Vec<Rc<SweepEvent>>) -> Vec<Contour> {
    let result_events = order_events(sorted_events);

    let iteration_map = precompute_iteration_order(&result_events);

    let mut contours: Vec<Contour> = vec![];
    let mut processed: HashSet<usize> = HashSet::new();

    for i in 0..result_events.len() {
        if processed.contains(&i) {
            continue;
        }

        let contour_id = contours.len();
        let mut contour =
            Contour::initialise_from_context(&result_events[i], &mut contours, contour_id);

        let mut pos = i;

        let initial = result_events[i].point;
        contour
            .edges
            .push((initial, initial, result_events[i].source.clone()));

        loop {
            processed.insert(pos);
            *result_events[pos].output_contour_id_mut() = contour_id;

            pos = result_events[pos].other_pos();

            processed.insert(pos);
            *result_events[pos].output_contour_id_mut() = contour_id;

            let new_point = result_events[pos].point;
            if contour.edges.last().map(|e| &e.2) != Some(&result_events[pos].source.clone()) {
                contour
                    .edges
                    .push((new_point, new_point, result_events[pos].source.clone()));
            } else {
                contour.edges.last_mut().unwrap().1 = new_point;
            }

            let next_pos_opt = get_next_pos(pos, &processed, &iteration_map);
            match next_pos_opt {
                Some(npos) => pos = npos,
                None => break,
            }

            if pos == i {
                break;
            }
        }

        contours.push(contour);
    }
    contours
}

fn order_events(sorted_events: Vec<Rc<SweepEvent>>) -> Vec<Rc<SweepEvent>> {
    let mut result_events: Vec<_> = sorted_events
        .into_iter()
        .filter(|event| {
            (event.left() && event.is_in_result())
                || (!event.left() && event.other().map(|o| o.is_in_result()).unwrap_or(false))
        })
        .collect();

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

pub fn precompute_iteration_order(data: &[Rc<SweepEvent>]) -> Vec<usize> {
    let mut map = vec![0; data.len()];

    let mut i = 0;
    while i < data.len() {
        dbg!(i, &data[i].point, data[i].left());
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
            for j in r_from..to {
                map[j] = j + 1;
            }
            if has_l {
                map[to] = l_to - 1;
            } else {
                map[to] = r_from;
            }
        }
        if has_l {
            let to = l_to - 1;
            for j in (l_from + 1)..=to {
                map[j] = j - 1;
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

fn get_next_pos(
    mut pos: usize,
    processed: &HashSet<usize>,
    iteration_map: &[usize],
) -> Option<usize> {
    let start_pos = pos;

    loop {
        let next = iteration_map[pos];
        dbg!(pos, next, processed.contains(&next));
        pos = iteration_map[pos];
        if pos == start_pos {
            return None;
        } else if !processed.contains(&pos) {
            return Some(pos);
        }
    }
}
