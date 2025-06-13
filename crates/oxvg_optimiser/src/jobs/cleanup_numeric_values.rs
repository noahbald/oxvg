use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

use crate::utils::cleanup_values::{self, CleanupValues, Mode};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Rounds number and removes default `px` unit in attributes specified with a number number.
///
/// # Correctness
///
/// Rounding errors may cause slight changes in visual appearance.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct CleanupNumericValues {
    #[serde(default = "default_float_precision")]
    /// The number of decimal places to round to
    pub float_precision: u8,
    #[serde(default = "default_leading_zero")]
    /// Whether to trim leading zeros
    pub leading_zero: bool,
    #[serde(default = "default_default_px")]
    /// Whether to remove `"px"` from the units
    pub default_px: bool,
    #[serde(default = "default_convert_to_px")]
    /// Whether to convert absolute units to `"px"`
    pub convert_to_px: bool,
}

impl Default for CleanupNumericValues {
    fn default() -> Self {
        CleanupNumericValues {
            float_precision: default_float_precision(),
            leading_zero: default_leading_zero(),
            default_px: default_default_px(),
            convert_to_px: default_convert_to_px(),
        }
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for CleanupNumericValues {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        CleanupValues::element(self, element, context)
    }
}

impl CleanupValues for CleanupNumericValues {
    fn get_options(&self) -> cleanup_values::Options {
        cleanup_values::Options {
            float_precision: self.float_precision as usize,
            leading_zero: self.leading_zero,
            default_px: self.default_px,
            do_convert_to_px: self.convert_to_px,
        }
    }

    fn get_mode(&self) -> Mode {
        Mode::SingleValue
    }
}

const fn default_float_precision() -> u8 {
    3
}
const fn default_leading_zero() -> bool {
    true
}
const fn default_default_px() -> bool {
    true
}
const fn default_convert_to_px() -> bool {
    true
}

#[test]
fn cleanup_numeric_values() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupNumericValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="20.000001 -19.99999 17.123456 70.708090" width="50.12356%" height="20px" x=".2655" y="-.2346">
    <!-- Should round values, maintaining non-numerical values -->
    <rect width="1in" height="12pt"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupNumericValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0, 0, 20, 20">
    <!-- Should round values, maintaining non-numerical values -->
    <rect width="20" height="20" fill="rgba(255,255,255,.85)" rx="20"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupNumericValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox=" 0 0      150 100 ">
    <!-- Should remove unnecessary whitespace from `viewBox -->
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupNumericValues": { "floatPrecision": 0 } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1052.4 744.1">
    <!-- Should round to zero decimal places -->
</svg>"#
        )
    )?);

    Ok(())
}
