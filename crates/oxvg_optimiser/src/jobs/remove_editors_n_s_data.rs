use std::collections::HashSet;

use oxvg_ast::{
    element::Element,
    visitor::{Context, Visitor},
};
use oxvg_collections::{
    attribute::{Attr, AttrId},
    name::{Prefix, QualName},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
        if is_editor_namespace(uri)
            || self
                .additional_namespaces
                .as_ref()
                .is_some_and(|set| set.contains(&**uri))
        {
            element.remove();
            return Ok(());
        }

        element.attributes().retain(|attr| {
            if let Attr::Unparsed { attr_id, value } = attr {
                if let AttrId::Unknown(QualName {
                    prefix: Prefix::XMLNS,
                    local: _,
                }) = &**attr_id
                {
                    return !is_editor_namespace(value)
                        && !self
                            .additional_namespaces
                            .as_ref()
                            .is_some_and(|set| set.contains(&**value));
                }
            }

            let uri = attr.prefix().ns().uri();
            !is_editor_namespace(uri)
                && !self
                    .additional_namespaces
                    .as_ref()
                    .is_some_and(|set| set.contains(&**uri))
        });

        Ok(())
    }
}

fn is_editor_namespace(uri: &str) -> bool {
    matches!(
        uri,
        "http://creativecommons.org/ns#"
            | "http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd"
            | "http://ns.adobe.com/AdobeIllustrator/10.0/"
            | "http://ns.adobe.com/AdobeSVGViewerExtensions/3.0/"
            | "http://ns.adobe.com/Extensibility/1.0/"
            | "http://ns.adobe.com/Flows/1.0/"
            | "http://ns.adobe.com/GenericCustomNamespace/1.0/"
            | "http://ns.adobe.com/Graphs/1.0/"
            | "http://ns.adobe.com/ImageReplacement/1.0/"
            | "http://ns.adobe.com/SaveForWeb/1.0/"
            | "http://ns.adobe.com/Variables/1.0/"
            | "http://ns.adobe.com/XPath/1.0/"
            | "http://purl.org/dc/elements/1.1/"
            | "http://schemas.microsoft.com/visio/2003/SVGExtensions/"
            | "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"
            | "http://taptrix.com/vectorillustrator/svg_extensions"
            | "http://www.bohemiancoding.com/sketch/ns"
            | "http://www.figma.com/figma/ns"
            | "http://www.inkscape.org/namespaces/inkscape"
            | "http://www.serif.com/"
            | "http://www.vector.evaxdesign.sk"
            | "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
    )
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
