use lightningcss::values::percentage::DimensionPercentage;
use oxvg_ast::{
    attribute::data::{presentation::LengthPercentage, uncategorised::ViewBox, AttrId},
    element::Element,
    get_attribute, has_attribute, is_element, set_attribute,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(transparent)]
/// Removes `width` and `height` from the `<svg>` and replaces it with `viewBox` if missing.
///
/// This job is the opposite of [`super::RemoveViewBox`] and should be disabled before
/// using this one.
///
/// # Correctness
///
/// This job may affect the appearance of the document if the width/height does not match
/// the view-box.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveDimensions(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveDimensions {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _info: &Info<'input, 'arena>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !is_element!(element, Svg) {
            return Ok(());
        }

        if has_attribute!(element, ViewBox) {
            element.remove_attribute(&AttrId::Width);
            element.remove_attribute(&AttrId::Height);
            return Ok(());
        }

        let width = get_attribute!(element, Width);
        let Some(LengthPercentage(DimensionPercentage::Dimension(width))) = width.as_deref() else {
            return Ok(());
        };
        let Some(width) = width.to_px() else {
            return Ok(());
        };

        let height = get_attribute!(element, Height);
        let Some(LengthPercentage(DimensionPercentage::Dimension(height))) = height.as_deref()
        else {
            return Ok(());
        };
        let Some(height) = height.to_px() else {
            return Ok(());
        };

        let view_box = ViewBox {
            min_x: 0.0,
            min_y: 0.0,
            width,
            height,
        };

        element.remove_attribute(&AttrId::Width);
        element.remove_attribute(&AttrId::Height);
        set_attribute!(element, ViewBox(view_box));

        Ok(())
    }
}

#[test]
fn remove_dimensions() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeDimensions": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100.5" height=".5" viewBox="0 0 100.5 .5">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDimensions": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50" viewBox="0 0 100 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDimensions": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" viewBox="0 0 100 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDimensions": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDimensions": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100.5" height="0.5">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeDimensions": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100px" height="50px">
    test
</svg>"#
        ),
    )?);

    Ok(())
}
