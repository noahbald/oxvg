use std::{cell::RefCell, collections::BTreeSet};

use oxvg_ast::{
    atom::Atom,
    attribute::{Attr, Attributes},
    element::Element,
};
use oxvg_derive::OptionalDefault;
use oxvg_selectors::collections::EDITOR_NAMESPACES;
use serde::Deserialize;

use crate::{Job, JobDefault};

#[derive(Deserialize, Clone, Default, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveEditorsNSData {
    additional_namespaces: Option<BTreeSet<String>>,
    #[serde(skip_deserializing)]
    prefixes: RefCell<BTreeSet<String>>,
}

impl Job for RemoveEditorsNSData {
    fn run(&self, element: &impl oxvg_ast::element::Element, _context: &super::Context) {
        self.collect_svg_namespace(element);
        self.remove_editor_attributes(element);
        self.remove_editor_element(element);
    }
}

impl RemoveEditorsNSData {
    fn collect_svg_namespace(&self, element: &impl Element) {
        if element.local_name() != "svg".into() {
            return;
        }

        let mut prefixes = self.prefixes.borrow_mut();
        let xmlns_atom = "xmlns".into();
        for xmlns in element
            .attributes()
            .iter()
            .filter(|a| a.prefix().is_some_and(|p| p == xmlns_atom))
        {
            let value = xmlns.value();
            let value = value.as_str();
            if !EDITOR_NAMESPACES.contains(value)
                && !self
                    .additional_namespaces
                    .as_ref()
                    .is_some_and(|n| n.contains(value))
            {
                continue;
            }

            let name = xmlns.local_name();
            log::debug!("Adding {name} to prefixes");
            prefixes.insert(name.to_string());
        }
    }

    fn remove_editor_attributes(&self, element: &impl Element) {
        let prefixes = self.prefixes.borrow();
        element.attributes().retain(|(prefix, _, _)| {
            let Some(prefix) = prefix else {
                return true;
            };

            !prefixes.contains(prefix)
        });
    }

    fn remove_editor_element(&self, element: &impl Element) {
        let Some(prefix) = element.prefix() else {
            return;
        };

        if self.prefixes.borrow().contains(prefix.as_ref()) {
            log::debug!("Removing element with prefix: {prefix}");
            element.remove();
        }
    }
}

#[test]
fn remove_editors_ns_data() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeEditorsNsData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd">
    <sodipodi:namedview>
        ...
    </sodipodi:namedview>

    <path d="..." sodipodi:nodetypes="cccc"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeEditorsNsData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:sodipodi="http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd">
    <sodipodi:namedview>
        ...
    </sodipodi:namedview>

    <path d="..." sodipodi:nodetypes="cccc"/>
</svg>"#
        ),
    )?);

    Ok(())
}
