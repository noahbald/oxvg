use std::rc::Rc;

use markup5ever::{Attribute, QualName};
use oxvg_utils::rcdom::is_root;
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddClassesToSVG {
    pub class_names: Option<Vec<String>>,
    pub class_name: Option<String>,
}

impl Job for AddClassesToSVG {
    fn run(&self, node: &Rc<rcdom::Node>) {
        use rcdom::NodeData::Element;

        if !is_root(node) {
            return;
        }

        let Element { attrs, name, .. } = &node.data else {
            return;
        };
        if name.local.to_string() != "svg" {
            return;
        }

        let class_names = match self.class_names.clone() {
            Some(class_names) => class_names,
            None => match self.class_name.clone() {
                Some(class_name) => vec![class_name],
                None => return,
            },
        };
        let class_names = class_names.join(" ");

        let attrs = &mut *attrs.borrow_mut();
        let attr = attrs
            .iter_mut()
            .find(|attr| attr.name.local.to_string() == "class");
        let Some(attr) = attr else {
            attrs.push(Attribute {
                name: QualName::new(None, "".into(), "class".into()),
                value: class_names.into(),
            });
            return;
        };
        attr.value = format!("{} {}", attr.value.to_string().trim(), class_names).into();
    }
}

#[test]
fn add_classes_to_svg() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        // Should add classes when passed as a classNames Array
        r#"{ "addClassesToSvg": {
            "classNames": ["mySvg", "size-big"]
        } }"#,
        None
    )?);

    insta::assert_snapshot!(test_config(
        // Should add class when passed as a className String
        r#"{ "addClassesToSvg": {
            "className": "mySvg"
        } }"#,
        None
    )?);

    insta::assert_snapshot!(test_config(
        // Should avoid adding existing classes
        r#"{ "addClassesToSvg": {
            "className": "mySvg size-big"
        } }"#,
        Some(r#"<svg xmlns="http://www.w3.org/2000/svg" class="mySvg">"#)
    )?);
    Ok(())
}
