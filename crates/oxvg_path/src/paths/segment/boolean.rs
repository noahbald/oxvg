use crate::paths::segment::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Union,
    Intersection,
    Difference,
}

impl Path {
    pub fn unite(&self, other: &Self, tolerance: Option<f64>) -> Path {
        self.boolean(Operation::Union, other, tolerance)
    }

    pub fn intersect(&self, other: &Self, tolerance: Option<f64>) -> Path {
        self.boolean(Operation::Intersection, other, tolerance)
    }

    pub fn difference(&self, other: &Self, tolerance: Option<f64>) -> Path {
        self.boolean(Operation::Difference, other, tolerance)
    }

    pub fn boolean(&self, operation: Operation, other: &Self, tolerance: Option<f64>) -> Path {
        // TODO: Martinez-Rueda algorithm
        //
        // TODO: 1. Use `Path::flatten` to receive a polygonal version of each path to the given tolerance.
        // TODO: 2. Operate MR Algorithm against the polygon.
        // TODO: 3. Apply polygon operations to the path's `LineTo`, `CurveTo`, and `ArcTo` commands.
        todo!()
    }
}
