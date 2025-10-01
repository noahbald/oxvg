use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Rounds number and removes default `px` unit in attributes specified with number lists.
///
/// # Correctness
///
/// Rounding errors may cause slight changes in visual appearance.
///
/// # Errors
///
/// When a float-precision greater than the maximum is given.
pub struct CleanupListOfValues {
    #[serde(default = "default_float_precision")]
    // WARN: Lightningcss will round values to 5 decimal places
    /// Number of decimal places to round floating point numbers to, to a maximum of 5.
    pub float_precision: u8,
    #[serde(default = "default_leading_zero")]
    #[deprecated(note = "This option has no effect; leading zeroes are always removed")]
    /// Whether to trim leading zeros.
    pub leading_zero: bool,
    #[serde(default = "default_default_px")]
    /// Whether to remove `px` from a number's unit.
    pub default_px: bool,
    #[serde(default = "default_convert_to_px")]
    /// Whether to convert absolute units like `cm` and `in` to `px`.
    pub convert_to_px: bool,
}

impl Default for CleanupListOfValues {
    fn default() -> Self {
        CleanupListOfValues {
            float_precision: default_float_precision(),
            leading_zero: default_leading_zero(),
            default_px: default_default_px(),
            convert_to_px: default_convert_to_px(),
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for CleanupListOfValues {
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
                .round(self.float_precision as i32, self.convert_to_px, true);
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
fn cleanup_list_of_values() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupListOfValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500.2132 500.213823642" enable-background="new 0 0 500.224551535 500.213262">
    <!-- Should round values, maintaining non-numerical values -->
    test
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupListOfValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should cleanup polygon points -->
    <polygon stroke-dasharray="200.22222 200.22522" points="413.74712,290.95212 290.75632  ,  343.89942 183.40744 ,263.8582523 199.05334,  130.871345 322.04442,77.92533 429.39122,157.96555 "/>
    test
    <g fill="none" stroke-dasharray="8, 8" stroke-width="3">
        <path d="M83 250c69-18 140-40 197-84 33-23 48-62 62-99 2-6 3-12 7-16"/>
        <path stroke-dasharray="none" stroke-linejoin="round" d="M83 250c29-34 57-72 97-94 33-13 69-10 104-11 22 1 45 2 65 13"/>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupListOfValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should cleanup x/y values -->
    <text x="23.2350 20.2268px 0.22356em 80.0005%" y="23.2350 20.2268px 0.22356em 80.0005%" dx="23.2350 20.2268px 0.22356em 80.0005%" dy="23.2350 20.2268px 0.22356em 80.0005%">
        test
    </text>
</svg>"#
        )
    )?);

    Ok(())
}
