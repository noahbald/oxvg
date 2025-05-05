use oxvg_ast::{
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
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
pub struct RemoveRasterImages(#[napi(js_name = "enabled")] pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveRasterImages {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
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
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if element.prefix().is_some() || element.local_name().as_ref() != "image" {
            return Ok(());
        }
        let xlink_href_name = E::Name::new(Some("xlink".into()), "href".into());
        let Some(xlink_href) = element.get_attribute(&xlink_href_name) else {
            return Ok(());
        };

        if RASTER_IMAGE.is_match(xlink_href.as_ref()) {
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
