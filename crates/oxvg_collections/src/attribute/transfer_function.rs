//! Transfer function attributes as specified in [filter-effects](https://www.w3.org/TR/filter-effects-1/#transfer-function-element-attributes)
use crate::enum_attr;

enum_attr!(
    /// Indicates the type of component transfer function.
    ///
    /// For some initial component, `C` (e.g. `feFuncR`), the transfer function will
    /// remap to a component `C'`; both in the interval `[0, 1]`
    ///
    /// [w3](https://www.w3.org/TR/filter-effects-1/#element-attrdef-fecomponenttransfer-type)
    TransferFunctionType {
        /// `C' = C`
        Identity: "identity",
        /// Linearly interpolates between values given in `tableValues`
        Table: "table",
        /// Steps between values given in `tableValues`
        Discrete: "discrete",
        /// With the attributes `slope`, and `intercept`, `C' = slope * C + intercept`
        Linear: "linear",
        /// With the attributes `amplitude`, `exponent`, and `offset`, `C' = amplitude * (C^exponent) + offset`
        Gamma: "gamma",
    }
);
