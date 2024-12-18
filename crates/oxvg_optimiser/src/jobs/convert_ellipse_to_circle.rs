use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Job, PrepareOutcome};

use super::ContextFlags;

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct ConvertEllipseToCircle(bool);

impl<E: Element> Job<E> for ConvertEllipseToCircle {
    fn prepare(
        &mut self,
        _document: &E::ParentChild,
        _context_flags: &ContextFlags,
    ) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::None
        } else {
            PrepareOutcome::Skip
        }
    }
}

impl<E: Element> Visitor<E> for ConvertEllipseToCircle {
    type Error = String;

    #[allow(clippy::similar_names)]
    fn element(&mut self, element: &mut E, _context: &Context<E>) -> Result<(), String> {
        let name = element.local_name();
        if name.as_ref() != "ellipse" {
            return Ok(());
        }

        let rx_name = &"rx".into();
        let ry_name = &"ry".into();
        let rx = element
            .get_attribute_local(rx_name)
            .map_or(String::from("0"), |attr| attr.to_string());
        let ry = element
            .get_attribute_local(ry_name)
            .map_or(String::from("0"), |attr| attr.to_string());

        if rx != ry && rx != "auto" && ry != "auto" {
            return Ok(());
        }
        let radius = if rx == "auto" { ry } else { rx };
        element.remove_attribute_local(rx_name);
        element.remove_attribute_local(ry_name);
        element.set_attribute_local("r".into(), radius.into());
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
