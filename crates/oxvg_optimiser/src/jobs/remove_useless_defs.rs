use oxvg_ast::{
    element::Element,
    has_attribute, is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::element::ElementInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(transparent))]
/// Removes unreferenced `<defs>` elements
///
/// # Differences to SVGO
///
/// Defs with a class attribute will be retained, as only useful ones should remain
/// after running `inline_styles`.
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
pub struct RemoveUselessDefs(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveUselessDefs {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if has_attribute!(element, Id | Class) {
            return Ok(());
        }

        let name = element.qual_name();
        if !is_element!(element, Defs | SolidColor)
            && !name.info().contains(ElementInfo::NonRendering)
        {
            return Ok(());
        }

        let mut useful_nodes: Vec<Element<'input, 'arena>> = Vec::new();
        collect_useful_nodes(element, &mut useful_nodes);

        if useful_nodes.is_empty() {
            element.remove();
            return Ok(());
        }

        element.replace_children(useful_nodes.into_iter().map(|e| *e));
        Ok(())
    }
}

fn collect_useful_nodes<'input, 'arena>(
    element: &Element<'input, 'arena>,
    useful_nodes: &mut Vec<Element<'input, 'arena>>,
) {
    element.children_iter().for_each(|child| {
        if is_element!(child, Style) || has_attribute!(child, Id | Class) {
            useful_nodes.push(child);
        } else {
            collect_useful_nodes(&child, useful_nodes);
        }
    });
}

impl Default for RemoveUselessDefs {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn remove_metadata() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessDefs": true }"#,
        Some(
            r#"<svg>
    <defs>
        <path d="..."/>
        <g>
            <path d="..." id="a"/>
        </g>
    </defs>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessDefs": true }"#,
        Some(
            r#"<svg>
    <linearGradient id="linear">
        <stop/>
        <stop/>
    </linearGradient>
    <radialGradient id="radial">
        <stop/>
        <stop/>
    </radialGradient>
    <pattern id="pattern">
        <rect/>
    </pattern>
    <clipPath id="clip">
        <path/>
    </clipPath>
    <mask id="mask">
        <rect/>
    </mask>
    <marker id="marker">
        <path/>
    </marker>
    <symbol id="symbol">
        <rect/>
    </symbol>
    <solidColor id="color"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessDefs": true }"#,
        Some(
            r"<svg>
    <linearGradient>
        <stop/>
        <stop/>
    </linearGradient>
    <radialGradient>
        <stop/>
        <stop/>
    </radialGradient>
    <pattern>
        <rect/>
    </pattern>
    <clipPath>
        <path/>
    </clipPath>
    <mask>
        <rect/>
    </mask>
    <marker>
        <path/>
    </marker>
    <symbol>
        <rect/>
    </symbol>
    <solidColor/>
    <path/>
</svg>"
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessDefs": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <rect fill="url(#a)" width="64" height="64"/>
    <symbol>
        <linearGradient id="a">
            <stop offset="5%" stop-color="gold" />
        </linearGradient>
    </symbol>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeUselessDefs": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <rect fill="url(#a)" width="64" height="64"/>
    <g>
        <linearGradient id="a">
            <stop offset="5%" stop-color="gold" />
        </linearGradient>
    </g>
</svg>"#
        ),
    )?);

    Ok(())
}
