use std::{cell::Cell, collections::HashSet};

use oxvg_ast::{
    atom::Atom,
    attribute::data::{Attr, AttrId},
    element::{data::ElementId, Element},
    name::{Prefix, QualName},
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
/// Removes `xmlns` prefixed elements that are never referenced by a qualified name.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveUnusedNS(pub bool);

#[derive(Default)]
struct State<'input> {
    unused_namespaces: Cell<HashSet<Atom<'input>>>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveUnusedNS {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State::default().start(&mut document.clone(), context.info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'input> {
    type Error = JobsError<'input>;

    fn document(
        &self,
        document: &Element<'input, 'arena>,
        _content: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut unused_namespaces = self.unused_namespaces.take();
        document.child_elements_iter().for_each(|e| {
            self.root_element(&e, &mut unused_namespaces);
        });
        self.unused_namespaces.set(unused_namespaces);
        Ok(())
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut unused_namespaces = self.unused_namespaces.take();
        if unused_namespaces.is_empty() {
            return Ok(());
        }
        let prefix = element.prefix();
        if !prefix.is_empty() {
            unused_namespaces.remove(&prefix.ns().uri());
        }

        for attr in element.attributes().into_iter() {
            let prefix = attr.prefix();
            if !prefix.is_empty() {
                unused_namespaces.remove(&prefix.ns().uri());
            }
        }

        self.unused_namespaces.set(unused_namespaces);
        Ok(())
    }

    fn exit_document(
        &self,
        document: &Element<'input, 'arena>,
        _context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut unused_namespaces = self.unused_namespaces.take();
        document.child_elements_iter().for_each(|e| {
            self.exit_root_element(&e, &mut unused_namespaces);
        });
        self.unused_namespaces.set(unused_namespaces);
        Ok(())
    }
}

impl<'input> State<'input> {
    fn root_element(
        &self,
        element: &Element<'input, '_>,
        unused_namespaces: &mut HashSet<Atom<'input>>,
    ) {
        if *element.qual_name() != ElementId::Svg {
            return;
        }

        for attr in element.attributes().into_iter() {
            if let Attr::Unparsed {
                attr_id:
                    AttrId::Unknown(QualName {
                        prefix: Prefix::XMLNS,
                        ..
                    }),
                value,
            } = &*attr
            {
                unused_namespaces.insert(value.clone());
            }
        }
    }

    fn exit_root_element(
        &self,
        element: &Element<'input, '_>,
        unused_namespaces: &mut HashSet<Atom<'input>>,
    ) {
        if *element.qual_name() != ElementId::Svg {
            return;
        }

        element.attributes().retain(|attr| {
            let Attr::Unparsed {
                attr_id:
                    AttrId::Unknown(QualName {
                        prefix: Prefix::XMLNS,
                        ..
                    }),
                value,
            } = attr
            else {
                return true;
            };
            !unused_namespaces.contains(value)
        });
    }
}

impl Default for RemoveUnusedNS {
    fn default() -> Self {
        Self(true)
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
