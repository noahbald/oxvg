use std::collections::BTreeSet;

use oxvg_ast::element::Element;
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Context, Job, JobDefault};

#[derive(Deserialize, Default, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
#[job_default(is_default = false)]
/// Adds to the `class` attribute of the root `<svg>` element, omitting duplicates
///
/// <div class="warning">Unlike SVGO, this may change the order of your classes</div>
pub struct AddClassesToSVG {
    pub class_names: Option<Vec<String>>,
    pub class_name: Option<String>,
}

impl Job for AddClassesToSVG {
    fn run<E: Element>(&self, element: &E, _context: &Context) {
        if !element.is_root() || element.local_name() != "svg".into() {
            return;
        }

        let class_localname = "class".into();
        let attr = element.get_attribute(&class_localname);
        let attr = attr.map(|a| a.to_string()).unwrap_or_default();
        let class = attr.split_whitespace();

        let class_names: BTreeSet<_> = match &self.class_names {
            Some(names) => class
                .chain(names.into_iter().flat_map(|s| s.split_whitespace()))
                .collect(),
            None => match &self.class_name {
                Some(name) => class.chain(name.split_whitespace()).collect(),
                None => return,
            },
        };
        let class_names = class_names.into_iter().collect::<Vec<_>>().join(" ");

        element.set_attribute(class_localname, class_names.into());
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
