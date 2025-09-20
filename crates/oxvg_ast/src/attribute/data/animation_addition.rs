use crate::enum_attr;

enum_attr!(Accumulate {
    None: "none",
    Sum: "sum",
});
enum_attr!(Additive {
    Replace: "replace",
    Sum: "sum",
});
