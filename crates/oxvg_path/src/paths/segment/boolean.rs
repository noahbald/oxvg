use std::{
    cmp::Ordering,
    collections::{BTreeMap, BinaryHeap, HashMap},
    ops::Bound,
};

use crate::{
    geometry::{Line, Point},
    paths::{
        events,
        segment::{Data, Path, Segment, Tolerance, ToleranceSquared},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Union,
    Intersection,
    Difference,
}

enum EdgeType {
    Normal,
    NonContributing,
    SameTransition,
    DifferentTransition,
}

impl EdgeType {
    fn new(operation: &Operation, is_background: bool, in_out: bool, other_in_out: bool) -> Self {
        match operation {
            Operation::Union => match (in_out, other_in_out) {
                (_, true) => Self::NonContributing,
                (false, false) => Self::SameTransition,
                (true, false) => Self::DifferentTransition,
            },
            Operation::Intersection => match (in_out, other_in_out) {
                (_, false) => Self::NonContributing,
                (false, true) => Self::SameTransition,
                (true, true) => Self::DifferentTransition,
            },
            Operation::Difference => {
                if is_background {
                    match (in_out, other_in_out) {
                        (_, true) => Self::NonContributing,
                        (false, false) => EdgeType::SameTransition,
                        (true, false) => EdgeType::DifferentTransition,
                    }
                } else {
                    match (in_out, other_in_out) {
                        (_, false) => Self::NonContributing,
                        (true, true) => Self::DifferentTransition,
                        (false, true) => Self::SameTransition,
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
struct ActiveKey {
    y_at_sweep: f64,
    end_x: f64,
    event: usize,
}
impl Eq for ActiveKey {}
impl PartialOrd for ActiveKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ActiveKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.y_at_sweep
            .partial_cmp(&other.y_at_sweep)
            .unwrap_or(Ordering::Equal)
            .then(
                self.end_x
                    .partial_cmp(&other.end_x)
                    .unwrap_or(Ordering::Equal),
            )
            .then(self.event.cmp(&other.event))
    }
}
impl ActiveKey {
    fn new(event: usize, events: &[SweepEvent], sweep_x: f64) -> Self {
        let line = &events[event].line;
        let y_at_sweep = if (line.end().x() - line.start().x()).abs() < 1e-10 {
            line.start().y()
        } else {
            let t = (sweep_x - line.start().x()) / (line.end().x() - line.start().x());
            line.start().y() + t * (line.end().y() - line.start().y())
        };
        ActiveKey {
            y_at_sweep,
            end_x: line.end().x(),
            event,
        }
    }
}

/// A reference to an edge in the shape
struct SweepEvent {
    /// Whether this event had been subdivided and replaced with new events
    stale: bool,
    /// Whether it belongs to the background or foreground shape
    is_background: bool,
    /// The segment index of the background or foreground shape
    segment: usize,
    /// The data index of the segment
    command: usize,
    /// The start and end point of the event
    line: Line,
    in_out: bool,
    other_in_out: bool,
    edge_type: EdgeType,
}

impl PartialEq for EventRef {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event
    }
}

impl Eq for EventRef {}

impl PartialOrd for EventRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventRef {
    fn cmp(&self, other: &Self) -> Ordering {
        point_order(&other.point, &self.point)
            .then(other.is_left.cmp(&self.is_left))
            .then(other.event.cmp(&self.event))
    }
}

/// A reference to
struct EventRef {
    point: Point,
    /// An index to a tracked cut
    event: usize,
    /// Whether this ref is the start or end point of a line
    is_left: bool,
}

impl SweepEvent {
    fn new(is_background: bool, segment: usize, command: usize, line: Line) -> Self {
        Self {
            is_background,
            segment,
            command,
            line,
            stale: false,
            in_out: false,
            other_in_out: false,
            edge_type: EdgeType::Normal,
        }
    }
}

impl Path {
    pub fn unite(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Union, other, tolerance)
    }

    pub fn unite_self(mut self, tolerance: &Tolerance) -> Path {
        let foreground = Self(self.0.drain(0..self.0.len() / 2).collect());
        self.unite(&foreground, tolerance)
    }

    pub fn intersect(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Intersection, other, tolerance)
    }

    pub fn difference(&self, other: &Self, tolerance: &Tolerance) -> Path {
        self.boolean(Operation::Difference, other, tolerance)
    }

    pub fn boolean(&self, operation: Operation, other: &Self, tolerance: &Tolerance) -> Path {
        let tolerance_squared = &tolerance.square();
        // Modified Martinez-Rueda algorithm
        // 1. Flatten paths to a polygonal representation
        let background = events::Path::from_segments(self, tolerance_squared);
        let foreground = events::Path::from_segments(other, tolerance_squared);

        // 2. Operate MR Algorithm against the polygon.
        let (rings, events) = background.boolean(operation, &foreground, tolerance);

        // 3. Apply polygon operations to the path by cutting up `LineTo`, `CurveTo`, and `ArcTo` commands.
        Path(
            rings
                .into_iter()
                .map(|ring| {
                    ring_to_segment(ring, &events, &background, &foreground, tolerance_squared)
                })
                .collect(),
        )
    }
}

impl events::Path {
    pub fn boolean(
        &self,
        operation: Operation,
        foreground: &Self,
        tolerance: &Tolerance,
    ) -> (Vec<Vec<usize>>, Vec<SweepEvent>) {
        let tolerance_squared = &tolerance.square();
        // The list of cut up commands in the paths
        let mut events = vec![];
        // A queue of the
        let mut queue = BinaryHeap::new();

        collect_events(self, true, &mut events, &mut queue);
        collect_events(foreground, false, &mut events, &mut queue);

        // Edges ordered by above/below
        let mut active: BTreeMap<ActiveKey, usize> = BTreeMap::new();
        let mut results = vec![];

        while let Some(event_ref) = queue.pop() {
            let event = event_ref.event;
            if events[event].stale {
                continue;
            };

            if event_ref.is_left {
                let key = ActiveKey::new(event, &events, event_ref.point.x());
                active.insert(key, event);

                let below = active.range(..key).next_back().map(|(_, e)| *e);
                let above = active
                    .range((Bound::Excluded(key), Bound::Unbounded))
                    .next()
                    .map(|(_, e)| *e);

                let (in_out, other_in_out) = if let Some(below) = below {
                    let below = &events[below];
                    if below.is_background == events[event].is_background {
                        (!below.in_out, below.other_in_out)
                    } else {
                        (below.other_in_out, !below.in_out)
                    }
                } else {
                    (false, false)
                };
                events[event].in_out = in_out;
                events[event].other_in_out = other_in_out;
                events[event].edge_type = EdgeType::new(
                    &operation,
                    events[event].is_background,
                    in_out,
                    other_in_out,
                );

                if let Some(above) = above {
                    compute_intersect(
                        event,
                        above,
                        &mut events,
                        &mut queue,
                        &mut active,
                        event_ref.point.x(),
                        tolerance_squared,
                    );
                }
                if let Some(below) = below {
                    compute_intersect(
                        below,
                        event,
                        &mut events,
                        &mut queue,
                        &mut active,
                        event_ref.point.x(),
                        tolerance_squared,
                    );
                }
            } else {
                match events[event].edge_type {
                    EdgeType::NonContributing => {}
                    _ => results.push(event),
                }
                let key = ActiveKey::new(event, &events, event_ref.point.x());
                let below = active.range(..key).next_back().map(|(_, e)| *e);
                let above = active
                    .range((Bound::Excluded(key), Bound::Unbounded))
                    .next()
                    .map(|(_, e)| *e);
                active.remove(&key);
                if let (Some(below), Some(above)) = (below, above) {
                    compute_intersect(
                        below,
                        above,
                        &mut events,
                        &mut queue,
                        &mut active,
                        event_ref.point.x(),
                        tolerance_squared,
                    );
                }
            }
        }

        (
            chain_edges(results, &events, tolerance, tolerance_squared),
            events,
        )
    }
}

fn collect_events(
    path: &events::Path,
    is_background: bool,
    events: &mut Vec<SweepEvent>,
    queue: &mut BinaryHeap<EventRef>,
) {
    for (i, segment) in path.0.iter().enumerate() {
        for (j, command) in segment.data.iter().enumerate() {
            command.for_each(|line| {
                let event = events.len();
                events.push(SweepEvent::new(is_background, i, j, *line));
                queue.push(EventRef {
                    point: *line.start(),
                    event,
                    is_left: true,
                });
                queue.push(EventRef {
                    point: *line.end(),
                    event,
                    is_left: false,
                });
            });
        }
    }
}

// TODO: Move this to `geometry::point`?
fn point_order(p1: &Point, p2: &Point) -> Ordering {
    match p1.x().partial_cmp(&p2.x()).unwrap_or(Ordering::Equal) {
        Ordering::Equal => p1.y().partial_cmp(&p2.y()).unwrap_or(Ordering::Equal),
        ord => ord,
    }
}

fn compute_intersect(
    a_segment: usize,
    b_segment: usize,
    events: &mut Vec<SweepEvent>,
    queue: &mut BinaryHeap<EventRef>,
    active: &mut BTreeMap<ActiveKey, usize>,
    sweep_x: f64,
    tolerance_squared: &ToleranceSquared,
) {
    let a = &events[a_segment];
    let b = &events[b_segment];

    if let Some((t, _u)) = // HELP: Why compute `_u`
        segment_intersect(a.line.start(), a.line.end(), b.line.start(), b.line.end())
    {
        let ip = *a.line.start() + t * (a.line.end() - a.line.start());
        if ip.distance_squared(a.line.start()) > **tolerance_squared
            && ip.distance_squared(a.line.end()) > **tolerance_squared
            && ip.distance_squared(b.line.start()) > **tolerance_squared
            && ip.distance_squared(b.line.end()) > **tolerance_squared
        {
            subdivide_at(a_segment, ip, events, queue, active, sweep_x);
            subdivide_at(b_segment, ip, events, queue, active, sweep_x);
        }
    }
}

fn segment_intersect(p1: &Point, p2: &Point, p3: &Point, p4: &Point) -> Option<(f64, f64)> {
    let d1 = p2 - p1;
    let d2 = p4 - p3;
    let cross = Point::cross(Point::ZERO, d1, d2);
    if cross.abs() < 1e-10 {
        return None;
    }
    let d3 = p3 - p1;
    let t = Point::cross(Point::ZERO, d3, d2) / cross;
    let u = Point::cross(Point::ZERO, d3, d1) / cross;
    if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
        Some((t, u))
    } else {
        None
    }
}

fn subdivide_at(
    event: usize,
    point: Point,
    events: &mut Vec<SweepEvent>,
    queue: &mut BinaryHeap<EventRef>,
    active: &mut BTreeMap<ActiveKey, usize>,
    sweep_x: f64,
) {
    active.remove(&ActiveKey::new(event, events, sweep_x));
    let is_background = events[event].is_background;
    let segment = events[event].segment;
    let command = events[event].command;
    let line = events[event].line;
    events[event].stale = true;

    let event_left = events.len();
    let event_right = event_left + 1;
    events.push(SweepEvent::new(
        is_background,
        segment,
        command,
        Line([*line.start(), point]),
    ));
    events.push(SweepEvent::new(
        is_background,
        segment,
        command,
        Line([point, *line.end()]),
    ));
    // Queue left half from start (left of left)
    queue.push(EventRef {
        event: event_left,
        point: *line.start(),
        is_left: true,
    });
    // Queue left half to subdivide point (right of left)
    queue.push(EventRef {
        event: event_left,
        point,
        is_left: false,
    });
    // Queue right half from subdivide point (left of right)
    queue.push(EventRef {
        event: event_right,
        point,
        is_left: true,
    });
    // Queue right half to end (right of right)
    queue.push(EventRef {
        event: event_right,
        point: *line.end(),
        is_left: false,
    });
}

fn quantise(p: &Point, tolerance: &Tolerance) -> (i64, i64) {
    (
        (p.x() / tolerance.positional).round() as i64,
        (p.y() / tolerance.positional).round() as i64,
    )
}

fn chain_edges(
    results: Vec<usize>,
    events: &[SweepEvent],
    tolerance: &Tolerance,
    tolerance_squared: &ToleranceSquared,
) -> Vec<Vec<usize>> {
    let mut by_start: HashMap<(i64, i64), Vec<usize>> = HashMap::with_capacity(results.len());
    for &event in &results {
        let effective_start = if matches!(events[event].edge_type, EdgeType::DifferentTransition) {
            events[event].line.end()
        } else {
            events[event].line.start()
        };
        by_start
            .entry(quantise(effective_start, tolerance))
            .or_default()
            .push(event);
    }
    let mut used = vec![false; events.len()];
    let mut rings = vec![];

    for &first in &results {
        if used[first] {
            continue;
        }
        let mut ring = vec![first];
        used[first] = true;
        let reversed = matches!(events[first].edge_type, EdgeType::DifferentTransition);
        let (start, mut end) = if reversed {
            (events[first].line.end(), events[first].line.start())
        } else {
            (events[first].line.start(), events[first].line.end())
        };

        loop {
            if end.distance_squared(&start) <= **tolerance_squared {
                break;
            }
            let key = quantise(&end, tolerance);
            let Some(candidates) = by_start.get_mut(&key) else {
                break;
            };
            let Some(pos) = candidates.iter().position(|&e| !used[e]) else {
                break;
            };
            let next = candidates[pos];
            used[next] = true;
            end = if reversed {
                events[next].line.start()
            } else {
                events[next].line.end()
            };
            ring.push(next)
        }
        debug_assert!(ring.len() >= 3);
        rings.push(ring);
    }
    rings
}

fn ring_to_segment(
    ring: Vec<usize>,
    events: &[SweepEvent],
    background: &events::Path,
    foreground: &events::Path,
    tolerance: &ToleranceSquared,
) -> Segment {
    let mut start = ring[0];
    let mut segment = Segment {
        start: if matches!(events[start].edge_type, EdgeType::DifferentTransition) {
            *events[start].line.end()
        } else {
            *events[start].line.start()
        },
        data: vec![],
        closed: true,
    };
    for i in 0..ring.len() {
        let current = &events[ring[i]];
        let next = &events[ring[(i + 1) % ring.len()]];
        let current_reversed = matches!(current.edge_type, EdgeType::DifferentTransition);
        let next_reversed = matches!(next.edge_type, EdgeType::DifferentTransition);
        let extends = i + 1 < ring.len()
            && current_reversed == next_reversed
            && current.is_background == next.is_background
            && current.segment == next.segment
            && current.command == next.command;
        if extends {
            continue;
        }

        let begin = &events[ring[start]];
        debug_assert_eq!(
            current_reversed,
            matches!(begin.edge_type, EdgeType::DifferentTransition)
        );
        let source = if current.is_background {
            background
        } else {
            foreground
        };
        let command = &source.0[current.segment].data[current.command];
        segment.data.push(slice_command(
            command,
            if current_reversed {
                current.line.end()
            } else {
                begin.line.start()
            },
            if current_reversed {
                begin.line.start()
            } else {
                current.line.end()
            },
            current_reversed,
            tolerance,
        ));
        start = i + 1;
    }
    segment
}

fn slice_command(
    command: &events::Data,
    start: &Point,
    end: &Point,
    reversed: bool,
    tolerance: &ToleranceSquared,
) -> Data {
    // NOTE: Exact equality ok, because event points are derived from
    //       path points.
    let command = match command {
        events::Data::Line(_) => Data::LineTo(*end),
        events::Data::Curve(curve, p) => {
            if start == p[0].start() && p.last().unwrap().end() == end {
                return Data::CurveTo(*curve);
            }

            let right = if start == p[0].start() {
                *curve
            } else {
                curve
                    .subdivide_at(*p[0].start(), *start, tolerance)
                    .unwrap()
                    .1
            };
            let middle = if end == p.last().unwrap().end() {
                right
            } else {
                right.subdivide_at(*start, *end, tolerance).unwrap().0
            };
            Data::CurveTo(middle)
        }
        events::Data::Arc(arc, p) => {
            if start == p[0].start() && p.last().unwrap().end() == end {
                return Data::ArcTo(*arc);
            }

            let t1 = if start == p[0].start() {
                0.0
            } else {
                arc.t_at(*start, tolerance).unwrap()
            };
            let t2 = if end == p.last().unwrap().end() {
                1.0
            } else {
                arc.t_at(*end, tolerance).unwrap()
            };
            Data::ArcTo(arc.clamp_t(t1, t2))
        }
    };
    if reversed {
        command.reverse(*start)
    } else {
        command
    }
}

#[cfg(test)]
mod test {
    use oxvg_parse::Parse;

    use crate::{
        paths::segment::{self, Tolerance},
        Path,
    };

    #[test]
    fn unite_squares() {
        let background = Path::parse_string("M0,0 L0,10 L10,10 L10,0 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance);
        assert_eq!(output.to_string(), "");
    }

    #[test]
    fn unite_squares_opposite_winding() {
        let background = Path::parse_string("M0,0 L10,0 L10,10 L0,10 L0,0").unwrap();
        let foreground = Path::parse_string("M5,5 L5,15 L15,15 L15,5 L5,5").unwrap();

        let tolerance = &Tolerance::default();
        let background = segment::Path::from_svg(&background, tolerance);
        let foreground = segment::Path::from_svg(&foreground, tolerance);

        let output = background
            .unite(&foreground, &Tolerance::default())
            .to_svg(tolerance);
        assert_eq!(output.to_string(), "");
    }
}
