use std::rc::Rc;

use markup5ever::local_name;
use oxvg_selectors::Element;
use serde::Deserialize;

use crate::{Context, Job, PrepareOutcome};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertEllipseToCircle(bool);

impl Job for ConvertEllipseToCircle {
    fn prepare(&mut self, _document: &rcdom::RcDom) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::None
        } else {
            PrepareOutcome::Skip
        }
    }

    fn run(&self, node: &Rc<rcdom::Node>, _context: &Context) {
        let element = Element::new(node.clone());
        let Some(name) = element.get_name() else {
            return;
        };
        if !matches!(name, local_name!("ellipse")) {
            return;
        }

        let rx = element
            .get_attr(&local_name!("rx"))
            .map_or_else(|| String::from("0"), |attr| String::from(attr.value));
        let ry = element
            .get_attr(&local_name!("ry"))
            .map_or_else(|| String::from("0"), |attr| String::from(attr.value));

        if rx != ry && rx != "auto" && ry != "auto" {
            return;
        }
        let radius = if rx == "auto" { ry } else { rx };
        element.remove_attr(&local_name!("rx"));
        element.remove_attr(&local_name!("ry"));
        element.set_attr(&local_name!("r"), radius.into());
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
