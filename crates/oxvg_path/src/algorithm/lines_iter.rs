use std::iter::Map;

use geo::{Coord, Line, LinesIter};
use itertools::{Itertools, TupleWindows};

use crate::algorithm::coords_iter::{ArcCoordsIter, CurveCoordsIter};

impl<'a> LinesIter<'a> for CurveCoordsIter<'a> {
    type Iter = Map<
        Map<
            TupleWindows<
                Map<CurveCoordsIter<'a>, fn(geo::Coord<f64>) -> (f64, f64)>,
                ((f64, f64), (f64, f64)),
            >,
            fn(((f64, f64), (f64, f64))) -> [(f64, f64); 2],
        >,
        fn([(f64, f64); 2]) -> Line<f64>,
    >;
    type Scalar = f64;

    fn lines_iter(&'a self) -> Self::Iter {
        self.clone()
            .map(<(f64, f64)>::from as fn(Coord) -> (f64, f64))
            .tuple_windows()
            .map(<[(f64, f64); 2]>::from as fn(((f64, f64), (f64, f64))) -> [(f64, f64); 2])
            .map(Line::from as fn([(f64, f64); 2]) -> Line)
    }
}

impl<'a> LinesIter<'a> for ArcCoordsIter<'a> {
    type Iter = Map<
        Map<
            TupleWindows<
                Map<ArcCoordsIter<'a>, fn(geo::Coord<f64>) -> (f64, f64)>,
                ((f64, f64), (f64, f64)),
            >,
            fn(((f64, f64), (f64, f64))) -> [(f64, f64); 2],
        >,
        fn([(f64, f64); 2]) -> Line<f64>,
    >;
    type Scalar = f64;

    fn lines_iter(&'a self) -> Self::Iter {
        self.clone()
            .map(<(f64, f64)>::from as fn(Coord) -> (f64, f64))
            .tuple_windows()
            .map(<[(f64, f64); 2]>::from as fn(((f64, f64), (f64, f64))) -> [(f64, f64); 2])
            .map(Line::from as fn([(f64, f64); 2]) -> Line)
    }
}
