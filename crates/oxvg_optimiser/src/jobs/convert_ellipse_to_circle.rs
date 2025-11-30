use oxvg_ast::{
    element::Element,
    get_attribute, is_element, remove_attribute, set_attribute,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    attribute::{presentation::LengthPercentage, uncategorised::Radius},
    element::ElementId,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::JobsError;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(transparent))]
/// Converts non-eccentric `<ellipse>` to `<circle>` elements.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct ConvertEllipseToCircle(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for ConvertEllipseToCircle {
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

    #[allow(clippy::similar_names)]
    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !is_element!(element, Ellipse) {
            return Ok(());
        }

        let rx = get_attribute!(element, RX);
        let ry = get_attribute!(element, RY);

        // Can be converted to ellipse when
        // - rx/ry are equal
        // - at least one of rx/ry are auto
        let radius = match rx.as_deref() {
            None | Some(Radius::Auto) => match ry.as_deref() {
                None | Some(Radius::Auto) => None,
                Some(Radius::LengthPercentage(ry)) => Some(ry),
            },
            Some(Radius::LengthPercentage(rx)) => match ry.as_deref() {
                None | Some(Radius::Auto) => Some(rx),
                Some(Radius::LengthPercentage(ry)) => {
                    if rx == ry {
                        Some(rx)
                    } else {
                        return Ok(());
                    }
                }
            },
        }
        .cloned();
        log::debug!("derived {radius:?} from {rx:?}, {ry:?}");

        drop(rx);
        drop(ry);
        remove_attribute!(element, RX);
        remove_attribute!(element, RY);
        let element = element.set_local_name(ElementId::Circle, &context.info.allocator);
        set_attribute!(
            element,
            RGeometry(radius.unwrap_or_else(|| LengthPercentage::px(0.0)))
        );
        Ok(())
    }
}

impl Default for ConvertEllipseToCircle {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn convert_ellipse_to_circle() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertEllipseToCircle": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Convert circular ellipses to circles -->
    <ellipse rx="5" ry="5"/>
    <ellipse rx="auto" ry="5"/>
    <ellipse rx="5" ry="auto"/>
    <ellipse />
</svg>"#
        )
    )?);

    Ok(())
}
