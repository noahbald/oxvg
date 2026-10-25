use std::{ops::RangeInclusive, simd::prelude::*};

use crate::simd::{Op, fallback};

type U8x = u8x16;
type Mask = mask8x16;
const U8_X: usize = 16;

pub struct Eq;
pub struct Ne;
impl Op<Mask> for Eq {
    fn select(in_set: Mask) -> Mask {
        in_set
    }
    fn select_scalar(in_set: bool) -> bool {
        in_set
    }
}
impl Op<Mask> for Ne {
    fn select(in_set: Mask) -> Mask {
        !in_set
    }
    fn select_scalar(in_set: bool) -> bool {
        !in_set
    }
}

pub fn find_mixed_proto<O: Op<Mask>, const N: usize, const R: usize>(
    haystack: &[u8],
    needles: [u8; N],
    ranges: [RangeInclusive<u8>; R],
) -> Option<usize> {
    let needles_simd = needles.map(U8x::splat);
    let ranges_simd: [(U8x, U8x); R] =
        std::array::from_fn(|i| (U8x::splat(*ranges[i].start()), U8x::splat(*ranges[i].end())));

    let chunks = haystack.chunks_exact(U8_X);
    let remainder = chunks.remainder();

    let mut chunk_offset = 0;
    for chunk in chunks {
        let chunk = U8x::from_slice(chunk);
        let mut in_set = Mask::splat(false);
        for &needle in &needles_simd {
            in_set |= chunk.simd_eq(needle);
        }
        for &(start, end) in &ranges_simd {
            in_set |= chunk.simd_ge(start) & chunk.simd_le(end);
        }

        let mask = O::select(in_set).to_bitmask();
        if mask != 0 {
            return Some(chunk_offset + mask.trailing_zeros() as usize);
        }
        chunk_offset += U8_X;
    }
    fallback::find_mixed_proto::<Mask, O, N, R>(remainder, needles, ranges)
        .map(|p| p + chunk_offset)
}

pub fn find_proto<O: Op<Mask>, const N: usize>(haystack: &[u8], needles: [u8; N]) -> Option<usize> {
    return find_mixed_proto::<O, N, 0>(haystack, needles, []);
}

pub fn find_range_proto<O: Op<Mask>, const N: usize>(
    haystack: &[u8],
    ranges: [RangeInclusive<u8>; N],
) -> Option<usize> {
    return find_mixed_proto::<O, 0, N>(haystack, [], ranges);
}
