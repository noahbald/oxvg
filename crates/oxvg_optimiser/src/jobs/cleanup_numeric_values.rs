use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

use crate::utils::cleanup_values::{self, CleanupValues, Mode};

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupNumericValues {
    #[serde(default = "default_float_precision")]
    float_precision: usize,
    #[serde(default = "default_leading_zero")]
    leading_zero: bool,
    #[serde(default = "default_default_px")]
    default_px: bool,
    #[serde(default = "default_convert_to_px")]
    convert_to_px: bool,
}

impl<E: Element> Visitor<E> for CleanupNumericValues {
    type Error = String;

    fn element(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), Self::Error> {
        CleanupValues::element(self, element, context)
    }
}

impl CleanupValues for CleanupNumericValues {
    fn get_options(&self) -> cleanup_values::Options {
        cleanup_values::Options {
            float_precision: self.float_precision,
            leading_zero: self.leading_zero,
            default_px: self.default_px,
            do_convert_to_px: self.convert_to_px,
        }
    }

    fn get_mode(&self) -> Mode {
        Mode::SingleValue
    }
}

const fn default_float_precision() -> usize {
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

    Ok(())
}
