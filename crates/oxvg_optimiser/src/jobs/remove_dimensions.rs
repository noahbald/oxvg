use oxvg_ast::{
    element::Element,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveDimensions(bool);

impl<E: Element> Visitor<E> for RemoveDimensions {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), Self::Error> {
        if element.prefix().is_some() || element.local_name().as_ref() != "svg" {
            return Ok(());
        }

        let view_box_name = "viewBox".into();
        let view_box = element.get_attribute_local(&view_box_name);
        if view_box.is_some() {
            drop(view_box);
            element.remove_attribute_local(&"width".into());
            element.remove_attribute_local(&"height".into());
            return Ok(());
        }
        drop(view_box);

        let width_name = &"width".into();
        let Some(width_attr) = element.get_attribute_local(width_name) else {
            return Ok(());
        };
        let width = width_attr.as_ref();

        let height_name = &"height".into();
        let Some(height_attr) = element.get_attribute_local(height_name) else {
            return Ok(());
        };
        let height = height_attr.as_ref();

        if width.parse::<f64>().is_err() || height.parse::<f64>().is_err() {
            return Ok(());
        }

        let view_box = format!("0 0 {width} {height}").into();
        drop(width_attr);
        drop(height_attr);

        element.remove_attribute_local(width_name);
        element.remove_attribute_local(height_name);
        element.set_attribute_local(view_box_name, view_box);

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
