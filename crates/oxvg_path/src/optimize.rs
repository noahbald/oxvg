//! Methods for optimizing SVG paths

use crate::{paths::segment, Path};

impl Path {
    // TODO: Optimisation options based on `StyleInfo`
    pub fn optimize(&self) -> Path {
        let segments = segment::Path::from(self);

        // TODO: boolean union each closed segment
        //       - Skip when markers are present

        // TODO: Skip removing any when marker-mids are present
        // TODO: Skip removing moves by zero when stroke and linecap are present
        // TODO: Skip replacing closing (or almost closing) move with `Z` when not safe to use `Z`
        // TODO: Add `Z` when `Z` was previous used and markers are present
        let segments = segments.simplify();

        Into::into(&segments)
    }
}
