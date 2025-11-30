use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// Rounds number and removes default `px` unit in attributes specified with a number number.
///
/// # Correctness
///
/// Rounding errors may cause slight changes in visual appearance.
///
/// # Errors
///
/// When a float-precision greater than the maximum is given.
pub struct CleanupNumericValues {
    #[cfg_attr(feature = "serde", serde(default = "default_float_precision"))]
    // WARN: Lightningcss will round values to 5 decimal places
    /// Number of decimal places to round floating point numbers to, to a maximum of 5.
    pub float_precision: u8,
    #[cfg_attr(feature = "serde", serde(default = "default_leading_zero"))]
    #[deprecated(note = "This option has no effect; leading zeroes are always removed")]
    /// Whether to trim leading zeros.
    pub leading_zero: bool,
    #[cfg_attr(feature = "serde", serde(default = "default_default_px"))]
    /// Whether to remove `px` from a number's unit.
    pub default_px: bool,
    #[cfg_attr(feature = "serde", serde(default = "default_convert_to_px"))]
    /// Whether to convert absolute units like `cm` and `in` to `px`.
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

impl<'input, 'arena> Visitor<'input, 'arena> for CleanupNumericValues {
    type Error = JobsError<'input>;

    fn document(
        &self,
        _document: &Element<'input, 'arena>,
        _context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if self.float_precision > 5 {
            Err(JobsError::CleanupValuesPrecision(self.float_precision))
        } else {
            Ok(())
        }
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        element.attributes().into_iter_mut().for_each(|mut attr| {
            attr.value_mut()
                .round(self.float_precision as i32, self.convert_to_px, false);
        });
        Ok(())
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
