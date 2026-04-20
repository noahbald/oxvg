use std::{collections::HashSet, rc::Rc};

use crate::{
    geometry::Point,
    paths::{
        events,
        segment::{
            boolean::sweep_event::{ResultTransition, Source, SweepEvent},
            Data, Segment, ToleranceSquared,
        },
    },
};

#[derive(Clone, Debug)]
pub struct Contour {
    pub points: Vec<Point>,
    pub sources: Vec<Source>,
    pub holes: Vec<usize>,
    pub hole_of: Option<usize>,
    pub depth: usize,
}

impl Contour {
    fn new(hole_of: Option<usize>, depth: usize) -> Self {
        Self {
            points: vec![],
            sources: vec![],
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
        debug_assert_eq!(self.points.len(), self.sources.len());
        let mut segment = Segment {
            start: self.points[0],
            data: vec![],
            closed: true,
        };
        let mut i = 0;
        loop {
            let mut j = i;
            while j + 1 < self.sources.len() && self.sources[i] == self.sources[j + 1] {
                j += 1;
            }

            let source = if self.sources[i].background {
                background
            } else {
                foreground
            };
            let source = &source.0[self.sources[i].polygon];
            let source = &source.data[self.sources[i].command];
            let start = self.points[i];
            let end = self.points[j];

            segment.data.push(match source {
                events::Data::Line(_) => Data::LineTo(end),
                events::Data::Curve(curve, p) => {
                    if start == *p[0].start() && *p.last().unwrap().end() == end {
                        Data::CurveTo(*curve)
                    } else {
                        let right = if start == *p[0].start() {
                            *curve
                        } else {
                            curve
                                .subdivide_at(*p[0].start(), start, tolerance)
                                .unwrap()
                                .1
                        };
                        let middle = if end == *p.last().unwrap().end() {
                            right
                        } else {
                            right.subdivide_at(start, end, tolerance).unwrap().0
                        };
                        Data::CurveTo(middle)
                    }
                }
                events::Data::Arc(arc, p) => {
                    if start == *p[0].start() && *p.last().unwrap().end() == end {
                        Data::ArcTo(*arc)
                    } else {
                        let t1 = if start == *p[0].start() {
                            0.0
                        } else {
                            arc.t_at(start, tolerance).unwrap()
                        };
                        let t2 = if end == *p.last().unwrap().end() {
                            1.0
                        } else {
                            arc.t_at(end, tolerance).unwrap()
                        };
                        Data::ArcTo(arc.clamp_t(t1, t2))
                    }
                }
            });

            if i + 1 < self.sources.len() {
                i += 1;
            } else {
                break;
            }
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
        contour.points.push(initial);
        contour.sources.push(result_events[i].source.clone());

        loop {
            processed.insert(pos);
            *result_events[pos].output_contour_id_mut() = contour_id;

            pos = result_events[pos].other_pos();

            processed.insert(pos);
            *result_events[pos].output_contour_id_mut() = contour_id;
            contour.points.push(result_events[pos].point);
            contour.sources.push(result_events[pos].source.clone());

            let next_pos_opt = get_next_pos(pos, &processed, &iteration_map);
            match next_pos_opt {
                Some(npos) => pos = npos,
                None => break,
            }

            if result_events[pos].point == initial {
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
        let x = &data[i];

        let r_from = i;
        while i < data.len() && x.point == data[i].point && !data[i].left() {
            i += 1;
        }
        let r_to = i;

        let l_from = i;
        while i < data.len() && x.point == data[i].point {
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
        pos = iteration_map[pos];
        if pos == start_pos {
            return None;
        } else if !processed.contains(&pos) {
            return Some(pos);
        }
    }
}
