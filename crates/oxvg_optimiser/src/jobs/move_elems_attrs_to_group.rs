use std::collections::BTreeMap;

use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::{INHERITABLE_ATTRS, PATH_ELEMS};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Move an element's attributes to it's enclosing group.
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
pub struct MoveElemsAttrsToGroup(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for MoveElemsAttrsToGroup {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        _info: &Info<'arena, E>,
        context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        context_flags.query_has_stylesheet(document);
        Ok(
            if self.0 && !context_flags.contains(ContextFlags::has_stylesheet) {
                PrepareOutcome::none
            } else {
                PrepareOutcome::skip
            },
        )
    }

    fn exit_element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let name = element.qual_name();
        if name.prefix().is_some() {
            return Ok(());
        }
        if name.local_name().as_ref() != "g" {
            return Ok(());
        }

        let children = element.children();
        if children.len() <= 1 {
            log::debug!("not moving attrs, only 1 or 0 children");
            return Ok(());
        }

        let every_child_is_path = children.iter().all(|e| {
            let child_name = e.qual_name();
            child_name.prefix().is_none() && PATH_ELEMS.contains(child_name.local_name().as_ref())
        });
        let mut common_attributes = get_common_attributes(&children);

        let transform_name = <E::Attr as Attr>::Name::new(None, "transform".into());
        if
        // preserve for other jobs
        every_child_is_path
            // preserve for pass-through attributes
            || element.has_attribute_local(&"filter".into())
            || element.has_attribute_local(&"clip-path".into())
            || element.has_attribute_local(&"mask".into())
        {
            common_attributes.remove(&transform_name);
        }

        for name in common_attributes.keys() {
            for child in &children {
                child.remove_attribute(name);
            }
        }
        for (name, value) in common_attributes {
            if name != transform_name {
                element.set_attribute(name, value);
                continue;
            }

            if let Some(mut attr) = element.get_attribute_node_mut(&transform_name) {
                let value = format!("{} {value}", attr.value());
                attr.set_value(value.into());
            } else {
                element.set_attribute(name, value);
            }
        }
        Ok(())
    }
}

fn get_common_attributes<'arena, E: Element<'arena>>(children: &[E]) -> BTreeMap<E::Name, E::Atom> {
    let mut child_iter = children.iter().map(Element::attributes);
    let mut common_attributes: BTreeMap<_, _> = child_iter
        .next()
        .expect("element should have >1 child")
        .into_iter()
        .filter(|a| INHERITABLE_ATTRS.contains(&a.name().formatter().to_string()))
        .map(|a| (a.name().clone(), a.value().clone()))
        .collect();
    child_iter.for_each(|attrs| {
        common_attributes.retain(|name, value| {
            attrs
                .get_named_item(name)
                .is_some_and(|a| a.value() == value)
        });
    });

    common_attributes
}

impl Default for MoveElemsAttrsToGroup {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn move_elems_attrs_to_group() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "moveElemsAttrsToGroup": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- move common attributes -->
    <g attr1="val1">
        <g fill="red" color="#000" stroke="blue">
            text
        </g>
        <g>
          <rect fill="red" color="#000" />
          <ellipsis fill="red" color="#000" />
        </g>
        <circle fill="red" color="#000" attr3="val3"/>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveElemsAttrsToGroup": true }"#,
        Some(
            r#"<svg>
    <!-- overwrite with child attributes -->
    <g fill="red">
        <rect fill="blue" />
        <circle fill="blue" />
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveElemsAttrsToGroup": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- move only common attributes -->
    <g attr1="val1">
        <g attr2="val2">
            text
        </g>
        <circle attr2="val2" attr3="val3"/>
        <path d="..."/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveElemsAttrsToGroup": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- preserve transform for masked/clipped groups -->
    <mask id="mask">
        <path/>
    </mask>
    <g transform="rotate(45)">
        <g transform="scale(2)" fill="red">
            <path d="..."/>
        </g>
        <circle fill="red" transform="scale(2)"/>
    </g>
    <g clip-path="url(#clipPath)">
        <g transform="translate(10 10)"/>
        <g transform="translate(10 10)"/>
    </g>
    <g mask="url(#mask)">
        <g transform="translate(10 10)"/>
        <g transform="translate(10 10)"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveElemsAttrsToGroup": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- preserve transform when all children are paths -->
    <g>
        <path transform="scale(2)" d="M0,0 L10,20"/>
        <path transform="scale(2)" d="M0,10 L20,30"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveElemsAttrsToGroup": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- don't run when style is present -->
    <style id="current-color-scheme">
        .ColorScheme-Highlight{color:#3daee9}
    </style>
    <g>
        <path transform="matrix(-1 0 0 1 72 51)" class="ColorScheme-Highlight" fill="currentColor" d="M5-28h26v2H5z"/>
        <path transform="matrix(-1 0 0 1 72 51)" class="ColorScheme-Highlight" fill="currentColor" d="M5-29h26v1H5z"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveElemsAttrsToGroup": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
    <!-- don't move if there is a filter attr on a group -->
    <defs>
        <filter id="a" x="17" y="13" width="12" height="10" filterUnits="userSpaceOnUse">
            <feGaussianBlur stdDeviation=".01"/>
        </filter>
    </defs>
    <g filter="url(#a)">
        <rect x="19" y="12" width="14" height="6" rx="3" transform="rotate(31 19 12.79)"/>
        <rect x="19" y="12" width="14" height="6" rx="3" transform="rotate(31 19 12.79)"/>
    </g>
</svg>"#
        ),
    )?);

    Ok(())
}
