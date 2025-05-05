use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

use crate::utils::cleanup_values::{self, CleanupValues, Mode};

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
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct CleanupListOfValues {
    #[serde(default = "default_float_precision")]
    /// Number of decimal places to round floating point numbers to.
    pub float_precision: u8,
    #[serde(default = "default_leading_zero")]
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

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for CleanupListOfValues {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        CleanupValues::element(self, element, context)
    }
}

impl CleanupValues for CleanupListOfValues {
    fn get_options(&self) -> cleanup_values::Options {
        cleanup_values::Options {
            float_precision: self.float_precision as usize,
            leading_zero: self.leading_zero,
            default_px: self.default_px,
            do_convert_to_px: self.convert_to_px,
        }
    }

    fn get_mode(&self) -> Mode {
        Mode::List
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
