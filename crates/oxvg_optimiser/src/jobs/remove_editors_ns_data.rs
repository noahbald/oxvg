use std::{collections::HashSet, marker::PhantomData};

use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    visitor::{Context, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::EDITOR_NAMESPACES;
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
/// Removes all xml namespaces associated with editing software.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// Editor namespaces may be used by the editor and contain data that might be
/// lost if you try to edit the file after optimising.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveEditorsNSData {
    /// A list of additional namespaces URIs you may want to remove.
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub additional_namespaces: Option<HashSet<String>>,
}

struct State<'o, 'arena, E: Element<'arena>> {
    options: &'o RemoveEditorsNSData,
    prefixes: HashSet<<E::Name as Name>::Prefix>,
    marker: PhantomData<&'arena ()>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveEditorsNSData {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        _context_flags: &mut oxvg_ast::visitor::ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        let mut state = State {
            options: self,
            prefixes: HashSet::new(),
            marker: PhantomData,
        };
        document.child_elements_iter().for_each(|ref e| {
            state.collect_svg_namespace(e);
        });
        state.start(&mut document.clone(), info, None)?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'_, 'arena, E> {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        self.remove_editor_attributes(element);
        self.remove_editor_element(element);
        Ok(())
    }
}

impl<'arena, E: Element<'arena>> State<'_, 'arena, E> {
    fn collect_svg_namespace(&mut self, element: &E) {
        if element.local_name().as_ref() != "svg" {
            return;
        }

        element.attributes().retain(|a| {
            if a.prefix().as_ref().is_none_or(|p| p.as_ref() != "xmlns") {
                return true;
            }
            let value = a.value().as_ref();
            if !EDITOR_NAMESPACES.contains(value)
                && !self
                    .options
                    .additional_namespaces
                    .as_ref()
                    .is_some_and(|set| set.contains(value))
            {
                return true;
            }

            let name = a.local_name();
            log::debug!("Adding {name} to prefixes");
            self.prefixes.insert(name.as_ref().into());
            false
        });
    }

    fn remove_editor_attributes(&self, element: &E) {
        element.attributes().retain(|attr| {
            let Some(prefix) = attr.prefix() else {
                return true;
            };

            !self.prefixes.contains(prefix)
        });
    }

    fn remove_editor_element(&self, element: &E) {
        let Some(prefix) = element.prefix() else {
            return;
        };

        if self.prefixes.contains(prefix) {
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
