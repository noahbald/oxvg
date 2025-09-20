use crate::enum_attr;

enum_attr!(TransferFunctionType {
    Identity: "identity",
    Table: "table",
    Discrete: "discrete",
    Linear: "linear",
    Gamma: "gamma",
});
