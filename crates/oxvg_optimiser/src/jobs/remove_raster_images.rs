use oxvg_ast::{
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRasterImages(pub bool);

impl<E: Element> Visitor<E> for RemoveRasterImages {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), Self::Error> {
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
