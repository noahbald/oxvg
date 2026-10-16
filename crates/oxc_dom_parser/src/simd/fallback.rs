use std::ops::RangeInclusive;

use crate::simd::Op;

pub struct Eq;
pub struct Ne;
impl Op<()> for Eq {
    fn select(in_set: ()) -> () {
        in_set
    }
    fn select_scalar(in_set: bool) -> bool {
        in_set
    }
}
impl Op<()> for Ne {
    fn select(in_set: ()) -> () {
        in_set
    }
    fn select_scalar(in_set: bool) -> bool {
        !in_set
    }
}

pub fn find_mixed_proto<T, O: Op<T>, const N: usize, const R: usize>(
    haystack: &[u8],
    needles: [u8; N],
    ranges: [RangeInclusive<u8>; R],
) -> Option<usize> {
    haystack
        .iter()
        .position(|c| O::select_scalar(needles.contains(c) || ranges.iter().any(|r| r.contains(c))))
}

pub fn find_proto<O: Op<()>, const N: usize>(haystack: &[u8], needles: [u8; N]) -> Option<usize> {
    return find_mixed_proto::<(), O, N, 0>(haystack, needles, []);
}

pub fn find_range_proto<O: Op<()>, const N: usize>(
    haystack: &[u8],
    ranges: [RangeInclusive<u8>; N],
) -> Option<usize> {
    return find_mixed_proto::<(), O, 0, N>(haystack, [], ranges);
}
