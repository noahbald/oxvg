use oxvg_ast::{
    atom::Atom,
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::{ElementGroup, Group};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoveUselessDefs(pub bool);

impl<E: Element> Visitor<E> for RemoveUselessDefs {
    type Error = String;

    fn prepare(
        &mut self,
        _document: &E,
        _context_flags: &mut ContextFlags,
    ) -> super::PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        let name = element.qual_name();
        if name.prefix().is_some() {
            return Ok(());
        }
        let name = name.local_name();
        if name.as_ref() != "defs"
            && (!ElementGroup::NonRendering.matches(name.as_str())
                || element.get_attribute_local(&"id".into()).is_some())
        {
            return Ok(());
        }

        let mut useful_nodes = vec![];
        collect_useful_nodes(element, &mut useful_nodes);

        if useful_nodes.is_empty() {
            element.remove();
            return Ok(());
        }

        element.replace_children(useful_nodes);
        Ok(())
    }
}

fn collect_useful_nodes<E: Element>(element: &E, useful_nodes: &mut Vec<E::Child>) {
    element.for_each_element_child(|child| {
        if child.prefix().is_none() && child.local_name().as_ref() == "style"
            || child.get_attribute_local(&"id".into()).is_some()
        {
            useful_nodes.push(child.as_child());
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
