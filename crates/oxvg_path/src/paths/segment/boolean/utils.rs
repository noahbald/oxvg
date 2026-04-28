use std::cmp::Ordering;

use crate::geometry::Point;

pub const fn less_if(c: bool) -> Ordering {
    if c {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

pub const fn inverse_less_if(c: bool) -> Ordering {
    less_if(!c)
}

pub fn signed_area(a: Point, b: Point, c: Point) -> f64 {
    robust::orient2d(
        robust::Coord { x: a.x(), y: a.y() },
        robust::Coord { x: b.x(), y: b.y() },
        robust::Coord { x: c.x(), y: c.y() },
    )
}
