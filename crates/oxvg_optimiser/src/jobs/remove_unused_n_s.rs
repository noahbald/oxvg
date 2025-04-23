use std::{cell::RefCell, collections::HashSet};

use derive_where::derive_where;
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct RemoveUnusedNS {
    enabled: bool,
}

#[derive_where(Default)]
struct State<'arena, E: Element<'arena>> {
    unused_namespaces: RefCell<HashSet<<E::Name as Name>::LocalName>>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveUnusedNS {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.enabled {
            State::<'arena, E>::default().start(&mut document.clone(), info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'arena, E> {
    type Error = String;

    fn document(
        &self,
        document: &mut E,
        _content: &Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        document.child_elements_iter().for_each(|e| {
            self.root_element(&e);
        });
        Ok(())
    }

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        let mut unused_namespaces = self.unused_namespaces.borrow_mut();
        if unused_namespaces.is_empty() {
            return Ok(());
        }
        if let Some(prefix) = element.prefix() {
            unused_namespaces.remove(&prefix.as_ref().into());
        }

        for attr in element.attributes().into_iter() {
            if let Some(prefix) = attr.prefix() {
                unused_namespaces.remove(&prefix.as_ref().into());
            }
        }

        Ok(())
    }

    fn exit_document(
        &self,
        document: &mut E,
        _context: &Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        document.child_elements_iter().for_each(|e| {
            self.exit_root_element(&e);
        });
        Ok(())
    }
}

impl<'arena, E: Element<'arena>> State<'arena, E> {
    fn root_element(&self, element: &E) {
        if element.prefix().is_none() && element.local_name().as_ref() == "svg" {
            let mut unused_namespaces = self.unused_namespaces.borrow_mut();
            for attr in element.attributes().into_iter() {
                if attr
                    .prefix()
                    .as_ref()
                    .is_some_and(|p| p.as_ref() == "xmlns")
                {
                    unused_namespaces.insert(attr.local_name().clone());
                }
            }
        }
    }

    fn exit_root_element(&self, element: &E) {
        if element.prefix().is_some() || element.local_name().as_ref() != "svg" {
            return;
        }

        for name in &*self.unused_namespaces.borrow() {
            log::debug!("removing xmlns:{name}");
            let name = E::Name::new(Some("xmlns".into()), name.clone());
            element.remove_attribute(&name);
        }
    }
}

impl Default for RemoveUnusedNS {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl<'de> Deserialize<'de> for RemoveUnusedNS {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let enabled = bool::deserialize(deserializer)?;
        Ok(Self { enabled })
    }
}

impl Serialize for RemoveUnusedNS {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.enabled.serialize(serializer)
    }
}

#[test]
fn remove_unused_n_s() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://test.com/">
    <g>
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://test.com/">
    <g test:attr="val">
        test
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://test.com/" xmlns:test2="http://test2.com/">
    <g test:attr="val">
        <g>
            test
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://test.com/" xmlns:test2="http://test2.com/">
    <g test:attr="val">
        <g test2:attr="val">
            test
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://test.com/" xmlns:test2="http://test2.com/">
    <g>
        <test:elem>
            test
        </test:elem>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUnusedNS": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:test="http://test.com/" xmlns:test2="http://test2.com/">
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
