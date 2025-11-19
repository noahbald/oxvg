use oxvg_ast::{
    element::Element,
    get_attribute, is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(transparent)]
/// Removes inline JPEGs, PNGs, and GIFs from the document.
///
/// # Correctness
///
/// This job may visually change documents with images inlined in them.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveRasterImages(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveRasterImages {
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
        if !is_element!(element, Image) {
            return Ok(());
        }
        let Some(xlink_href) = get_attribute!(element, XLinkHref) else {
            return Ok(());
        };

        if RASTER_IMAGE.is_match(&xlink_href) {
            element.remove();
        }
        Ok(())
    }
}

lazy_static! {
    static ref RASTER_IMAGE: regex::Regex =
        regex::Regex::new(r"(\.|image\/)(jpe?g|png|gif)").unwrap();
}

#[test]
fn remove_raster_images() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeRasterImages": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <g>
        <image xlink:href="raster.jpg" width="100" height="100"/>
        <image xlink:href="raster.png" width="100" height="100"/>
        <image xlink:href="raster.gif" width="100" height="100"/>
        <image xlink:href="raster.svg" width="100" height="100"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeRasterImages": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <g>
        <image xlink:href="data:image/jpg;base64,..." width="100" height="100"/>
        <image xlink:href="data:image/png;base64,..." width="100" height="100"/>
        <image xlink:href="data:image/gif;base64,..." width="100" height="100"/>
        <image xlink:href="data:image/svg+xml;base64,..." width="100" height="100"/>
    </g>
</svg>"#
        ),
    )?);

    Ok(())
}
