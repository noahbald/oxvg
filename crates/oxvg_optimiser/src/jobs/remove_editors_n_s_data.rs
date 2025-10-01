use std::collections::HashSet;

use oxvg_ast::{
    attribute::data::{Attr, AttrId},
    element::Element,
    name::{Prefix, QualName},
    visitor::{Context, Visitor},
};
use oxvg_collections::collections::EDITOR_NAMESPACES;
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

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

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveEditorsNSData {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let uri = element.prefix().ns().uri();
        if EDITOR_NAMESPACES.contains(uri)
            || self
                .additional_namespaces
                .as_ref()
                .is_some_and(|set| set.contains(&**uri))
        {
            element.remove();
            return Ok(());
        }

        element.attributes().retain(|attr| {
            if let Attr::Unparsed {
                attr_id:
                    AttrId::Unknown(QualName {
                        prefix: Prefix::XMLNS,
                        local: _,
                    }),
                value,
            } = attr
            {
                return !EDITOR_NAMESPACES.contains(value)
                    && !self
                        .additional_namespaces
                        .as_ref()
                        .is_some_and(|set| set.contains(&**value));
            };

            let uri = attr.prefix().ns().uri();
            !EDITOR_NAMESPACES.contains(uri)
                && !self
                    .additional_namespaces
                    .as_ref()
                    .is_some_and(|set| set.contains(&**uri))
        });

        Ok(())
    }
}

#[test]
fn remove_editors_n_s_data() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeEditorsNSData": {} }"#,
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
        r#"{ "removeEditorsNSData": {} }"#,
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
