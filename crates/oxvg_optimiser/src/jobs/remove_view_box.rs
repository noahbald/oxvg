use lightningcss::values::percentage::DimensionPercentage;
use oxvg_ast::{
    atom::Atom,
    attribute::{content_type::ContentType, data::presentation::LengthPercentage},
    element::Element,
    get_attribute, is_element, node, remove_attribute,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Removes the `viewBox` attribute when it matches the `width` and `height`.
///
/// # Correctness
///
/// This job should never visually change the document but may affect how the document
/// scales in applications.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveViewBox(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveViewBox {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
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
        if !is_element!(element, Pattern | Svg | Symbol) {
            return Ok(());
        }

        let Some(view_box) = get_attribute!(element, ViewBox) else {
            return Ok(());
        };
        let Some(width_attr) = element.get_attribute_local(&Atom::Static("width")) else {
            return Ok(());
        };
        let ContentType::LengthPercentage(width) = width_attr.value() else {
            return Ok(());
        };
        let LengthPercentage(DimensionPercentage::Dimension(width)) = &*width else {
            return Ok(());
        };
        let Some(width) = width.to_px() else {
            return Ok(());
        };
        drop(width_attr);

        let height_attr = element.get_attribute_local(&Atom::Static("height"));
        let Some(height) = height_attr.as_deref() else {
            return Ok(());
        };
        let ContentType::LengthPercentage(height) = height.value() else {
            return Ok(());
        };
        let LengthPercentage(DimensionPercentage::Dimension(height)) = &*height else {
            return Ok(());
        };
        let Some(height) = height.to_px() else {
            return Ok(());
        };
        drop(height_attr);

        if is_element!(element, Svg)
            && element
                .parent_node()
                .is_some_and(|n| n.node_type() != node::Type::Document)
        {
            // TODO: remove width/height for such case instead
            log::debug!("not removing viewbox from root svg");
            return Ok(());
        }

        if view_box.min_x == 0.0
            && view_box.min_y == 0.0
            && view_box.width == width
            && view_box.height == height
        {
            log::debug!("removing viewBox from element");
            drop(view_box);
            remove_attribute!(element, ViewBox);
        }

        Ok(())
    }
}

impl Default for RemoveViewBox {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_view_box() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100.5" height=".5" viewBox="0 0 100.5 .5">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50" viewBox="0 0 100 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0, 0, 100, 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50" viewBox="-25 -25 50 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r##"<svg width="480" height="360" viewBox="0 0 480 360" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <defs>
    <svg id="svg-sub-root" viewBox="0 0 450 450" width="450" height="450">
      <rect x="225" y="0" width="220" height="220" style="fill:magenta"/>
      <rect x="0" y="225" width="220" height="220" style="fill:#f0f"/>
      <rect x="225" y="225" width="220" height="220" fill="#f0f"/>
    </svg>
  </defs>
  <use x="60" y="50" width="240" height="240" xlink:href="#svg-sub-root"/>
  <rect x="300" y="170" width="118" height="118" fill="magenta"/>
</svg>"##
        ),
    )?);

    Ok(())
}
