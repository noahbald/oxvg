use std::{cell::RefCell, collections::BTreeSet};

use oxvg_ast::{
    atom::Atom,
    attribute::{Attr, Attributes},
    element::Element,
    visitor::{Context, Visitor},
};
use oxvg_collections::collections::EDITOR_NAMESPACES;
use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveEditorsNSData {
    additional_namespaces: Option<BTreeSet<String>>,
    #[serde(skip_deserializing)]
    prefixes: RefCell<BTreeSet<String>>,
}

impl<E: Element> Visitor<E> for RemoveEditorsNSData {
    type Error = String;

    fn document(
        &mut self,
        document: &mut E,
        _context: &Context<'_, '_, E>,
    ) -> Result<(), Self::Error> {
        document.for_each_element_child(|ref e| {
            self.collect_svg_namespace(e);
            self.remove_editor_attributes(e);
            self.remove_editor_element(e);
        });
        Ok(())
    }
}

impl RemoveEditorsNSData {
    fn collect_svg_namespace<E: Element>(&self, element: &E) {
        if element.local_name().as_ref() != "svg" {
            return;
        }

        let mut prefixes = self.prefixes.borrow_mut();
        for xmlns in element
            .attributes()
            .into_iter()
            .filter(|a| a.prefix().as_ref().is_some_and(|p| p.as_ref() == "xmlns"))
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

    fn remove_editor_attributes<E: Element>(&self, element: &E) {
        let prefixes = self.prefixes.borrow();
        element.attributes().retain(|attr| {
            let Some(prefix) = attr.prefix() else {
                return true;
            };

            !prefixes.contains(prefix.as_ref())
        });
    }

    fn remove_editor_element<E: Element>(&self, element: &E) {
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
