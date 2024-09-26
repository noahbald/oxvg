use std::{collections::BTreeSet, rc::Rc};

use markup5ever::{local_name, tendril::Tendril};
use serde::Deserialize;

use crate::{Context, Job};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
/// Adds to the `class` attribute of the root `<svg>` element, omitting duplicates
///
/// <div class="warning">Unlike SVGO, this may change the order of your classes</div>
pub struct AddClassesToSVG {
    pub class_names: Option<Vec<String>>,
    pub class_name: Option<String>,
}

impl Job for AddClassesToSVG {
    fn run(&self, node: &Rc<rcdom::Node>, _context: &Context) {
        let element = oxvg_selectors::Element::from(node);
        if !element.is_root() || element.get_name() != Some(local_name!("svg")) {
            return;
        }
        let class = element
            .get_attr(&local_name!("class"))
            .map(|attr| attr.value);
        let class = class.unwrap_or_else(Tendril::new);
        let class = class.split_whitespace().map(String::from);

        let class: BTreeSet<_> = match self.class_names.clone() {
            Some(names) => class
                .chain(names.into_iter().flat_map(|string| {
                    string
                        .split_whitespace()
                        .map(String::from)
                        .collect::<Vec<_>>()
                }))
                .collect(),
            None => match self.class_name.clone() {
                Some(name) => class
                    .chain(name.split_whitespace().map(String::from))
                    .collect(),
                None => return,
            },
        };
        let class_names = class.into_iter().collect::<Vec<_>>().join(" ");
        dbg!(&class_names);

        element.set_attr(&local_name!("class"), class_names.into());
    }
}

#[test]
fn add_classes_to_svg() -> anyhow::Result<()> {
    use crate::{test_config, test_config_default_svg_comment};

    insta::assert_snapshot!(test_config_default_svg_comment(
        r#"{ "addClassesToSvg": {
            "classNames": ["mySvg", "size-big"]
        } }"#,
        "Should add classes when passed as a classNames Array"
    )?);

    insta::assert_snapshot!(test_config_default_svg_comment(
        r#"{ "addClassesToSvg": {
            "className": "mySvg"
        } }"#,
        "Should add class when passed as a className String"
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "addClassesToSvg": {
            "className": "mySvg size-big"
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" class="mySvg">
    <!-- Should avoid adding existing classes -->
    test
</svg>"#
        )
    )?);
    Ok(())
}
