use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use serde::Deserialize;

use crate::utils::cleanup_values::{self, CleanupValues, Mode};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupNumericValues {
    float_precision: Option<usize>,
    leading_zero: Option<bool>,
    default_px: Option<bool>,
    convert_to_px: Option<bool>,
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
            float_precision: self.float_precision.unwrap_or(DEFAULT_FLOAT_PRECISION),
            leading_zero: self.leading_zero.unwrap_or(DEFAULT_LEADING_ZERO),
            default_px: self.default_px.unwrap_or(DEFAULT_DEFAULT_PX),
            do_convert_to_px: self.convert_to_px.unwrap_or(DEFAULT_CONVERT_TO_PX),
        }
    }

    fn get_mode(&self) -> Mode {
        Mode::SingleValue
    }
}

static DEFAULT_FLOAT_PRECISION: usize = 3;
static DEFAULT_LEADING_ZERO: bool = true;
static DEFAULT_DEFAULT_PX: bool = true;
static DEFAULT_CONVERT_TO_PX: bool = true;

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
