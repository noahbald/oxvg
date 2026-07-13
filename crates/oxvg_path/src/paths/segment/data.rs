use std::{
    cell::OnceCell,
    ops::{Deref, DerefMut},
};

use crate::{
    command::{self, ID},
    geometry::{Arc, Curve, Point, Tolerance, TolerancePrecision, ToleranceSquared},
    paths::segment::Path,
};

#[derive(Debug, PartialEq, Clone)]
/// A reduced representation of an SVG path command
pub enum Data {
    /// A line command
    LineTo(Point),
    /// A bezier command
    CurveTo(Curve),
    /// An arc command
    ArcTo(Arc),
}

#[derive(Debug, Clone)]
/// A wrapper of data that caches results of `to_svg` and `to_string` to prevent
/// redundant computation while simplifying.
pub struct CachedData {
    data: Data,
    cache: OnceCell<crate::command::CachedData>,
    control: OnceCell<Option<Point>>,
}

impl Deref for CachedData {
    type Target = Data;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl DerefMut for CachedData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cache.take();
        &mut self.data
    }
}
impl PartialEq for CachedData {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

impl Data {
    /// Returns data wrapped with auto-cacher
    pub fn with_cache(self) -> CachedData {
        CachedData {
            data: self,
            cache: OnceCell::new(),
            control: OnceCell::new(),
        }
    }

    /// Returns the end point of the data item.
    pub fn end_point(&self) -> Point {
        match self {
            Self::LineTo(point) => *point,
            Self::CurveTo(curve) => curve.end_point,
            Self::ArcTo(arc) => arc.end_point(),
        }
    }

    /// Returns the equivalent data item going from the end to the start.
    #[must_use]
    pub fn reverse(&self, start: Point) -> Self {
        match self {
            Data::LineTo(_) => Data::LineTo(start),
            Data::CurveTo(curve) => {
                Data::CurveTo(Curve::new(curve.end_control, curve.start_control, start))
            }
            Data::ArcTo(arc) => Data::ArcTo(
                Arc::new(
                    arc.center(),
                    arc.radii(),
                    arc.start_angle() + arc.sweep_angle(),
                    -arc.sweep_angle(),
                    arc.x_rotation(),
                )
                .with_end_point_memo(start),
            ),
        }
    }
}

impl CachedData {
    /// Wraps segment data with auto-caching segment data.
    pub fn new(data: Data) -> Self {
        Self {
            data,
            cache: OnceCell::new(),
            control: OnceCell::new(),
        }
    }

    /// Converts data to svg data, caching the result.
    ///
    /// See [`Path::to_svg`].
    ///
    /// # Panics
    ///
    /// If the cache is invalid.
    #[allow(clippy::too_many_arguments)]
    pub fn to_svg(
        &self,
        previous: Option<&command::CachedData>,
        next: Option<&Self>,
        segment_start: Point,
        start: Point,
        control: Option<Point>,
        last_control: &mut Option<Point>,
        implicit: Option<&ID>,
        tolerance: &Tolerance,
        tolerance_squared: ToleranceSquared,
        precision: TolerancePrecision,
        smart_arc_rounding: bool,
    ) -> command::CachedData {
        if let Some(data) = self.cache.get().cloned() {
            data
        } else {
            let data = self.data.to_svg(
                previous,
                next.map(Deref::deref),
                segment_start,
                start,
                control,
                last_control,
                implicit,
                tolerance,
                tolerance_squared,
                precision,
                smart_arc_rounding,
            );
            self.cache.set(data.clone()).unwrap();
            data
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn to_svg_curve(
        &self,
        previous: Option<&command::CachedData>,
        control: Option<Point>,
        start: Point,
        curve: &Curve,
        next: Option<&Self>,
        implicit: Option<&ID>,
        tolerance_squared: ToleranceSquared,
        precision: TolerancePrecision,
    ) -> (command::CachedData, Option<Point>) {
        if let Some(data) = self.cache.get().cloned() {
            (data, *self.control.get().unwrap())
        } else {
            let (data, control) = Path::to_svg_curve(
                previous,
                control,
                start,
                curve,
                next.map(Deref::deref),
                implicit,
                tolerance_squared,
                precision,
            );
            self.cache.set(data.clone()).unwrap();
            self.control.set(control).unwrap();
            (data, control)
        }
    }
}
