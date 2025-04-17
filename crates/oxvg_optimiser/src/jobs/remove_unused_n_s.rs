use std::collections::HashSet;

use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::Deserialize;

#[derive(Clone)]
pub struct RemoveUnusedNS {
    enabled: bool,
    unused_namespaces: HashSet<String>,
}

impl<E: Element> Visitor<E> for RemoveUnusedNS {
    type Error = String;

    fn prepare(
        &mut self,
        _document: &E,
        _context_flags: &mut oxvg_ast::visitor::ContextFlags,
    ) -> oxvg_ast::visitor::PrepareOutcome {
        if self.enabled {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn document(
        &mut self,
        document: &mut E,
        _content: &Context<'_, '_, E>,
    ) -> Result<(), Self::Error> {
        document.for_each_element_child(|e| {
            self.root_element(&e);
        });
        Ok(())
    }

    fn exit_document(
        &mut self,
        document: &mut E,
        _context: &Context<E>,
    ) -> Result<(), Self::Error> {
        document.for_each_element_child(|e| {
            self.exit_root_element(&e);
        });
        Ok(())
    }
}

impl RemoveUnusedNS {
    fn root_element<E: Element>(&mut self, element: &E) {
        if element.prefix().is_none() && element.local_name().as_ref() == "svg" {
            for attr in element.attributes().into_iter() {
                if attr
                    .prefix()
                    .as_ref()
                    .is_some_and(|p| p.as_ref() == "xmlns")
                {
                    self.unused_namespaces.insert(attr.local_name().to_string());
                }
            }
        }
        if self.unused_namespaces.is_empty() {
            return;
        }
        let Some(prefix) = element.prefix().as_ref().map(ToString::to_string) else {
            return;
        };

        self.unused_namespaces.remove(&prefix);
        for attr in element.attributes().into_iter() {
            if let Some(prefix) = attr.prefix().as_ref().map(ToString::to_string) {
                self.unused_namespaces.remove(&prefix);
            }
        }
    }

    fn exit_root_element<E: Element>(&self, element: &E) {
        if element.prefix().is_some() || element.local_name().as_ref() != "svg" {
            return;
        }

        for name in &self.unused_namespaces {
            log::debug!("removing xmlns:{name}");
            let name = E::Name::new(Some("xmlns".into()), name.as_str().into());
            element.remove_attribute(&name);
        }
    }
}

impl Default for RemoveUnusedNS {
    fn default() -> Self {
        Self {
            enabled: true,
            unused_namespaces: HashSet::new(),
        }
    }
}

impl<'de> Deserialize<'de> for RemoveUnusedNS {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let enabled = bool::deserialize(deserializer)?;
        Ok(Self {
            enabled,
            unused_namespaces: HashSet::new(),
        })
    }
}

#[test]
fn remove_unused_n_s() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://trololololololololololo.com/">
    <g>
        test
    </g>
</svg>"#
        ),
    )?);

    // FIXME: rcdom removes used xmlns
    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://trololololololololololo.com/">
    <g test:attr="val">
        test
    </g>
</svg>"#
        ),
    )?);

    // FIXME: rcdom removes used xmlns
    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://trololololololololololo.com/" xmlns:test2="http://trololololololololololo.com/">
    <g test:attr="val">
        <g>
            test
        </g>
    </g>
</svg>"#
        ),
    )?);

    // FIXME: rcdom removes used xmlns
    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://trololololololololololo.com/" xmlns:test2="http://trololololololololololo.com/">
    <g test:attr="val">
        <g test2:attr="val">
            test
        </g>
    </g>
</svg>"#
        ),
    )?);

    // FIXME: rcdom removes used xmlns
    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://trololololololololololo.com/" xmlns:test2="http://trololololololololololo.com/">
    <g>
        <test:elem>
            test
        </test:elem>
    </g>
</svg>"#
        ),
    )?);

    // FIXME: rcdom removes used xmlns
    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://trololololololololololo.com/" xmlns:test2="http://trololololololololololo.com/">
    <test:elem>
        <test2:elem>
            test
        </test2:elem>
    </test:elem>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" inkscape:version="0.92.2 (5c3e80d, 2017-08-06)" sodipodi:docname="test.svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd">
    test
</svg>"#
        ),
    )?);

    Ok(())
}
