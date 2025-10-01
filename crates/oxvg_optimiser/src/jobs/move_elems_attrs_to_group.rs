use std::collections::BTreeMap;

use oxvg_ast::{
    attribute::{
        data::{
            inheritable::{self, Inheritable},
            Attr, AttrId,
        },
        AttributeInfo,
    },
    element::Element,
    get_attribute_mut, has_attribute, is_attribute, is_element,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

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

impl<'input, 'arena> Visitor<'input, 'arena> for MoveElemsAttrsToGroup {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        Ok(
            if self.0
                && !context
                    .flags
                    .contains(ContextFlags::query_has_stylesheet_result)
            {
                PrepareOutcome::none
            } else {
                PrepareOutcome::skip
            },
        )
    }

    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !is_element!(element, G) {
            return Ok(());
        }

        if element.child_elements_iter().nth(1).is_none() {
            log::debug!("not moving attrs, only 1 or 0 children");
            return Ok(());
        }

        let every_child_is_path = element
            .child_elements_iter()
            .all(|e| e.qual_name().expected_attributes().contains(&AttrId::D));
        let mut common_attributes = get_common_attributes(element);

        if
        // preserve for other jobs
        every_child_is_path
            // preserve for pass-through attributes
            || has_attribute!(element, Filter)
            || has_attribute!(element, ClipPath)
            || has_attribute!(element, Mask)
        {
            common_attributes.remove(&AttrId::Transform);
        }

        for name in common_attributes.keys() {
            for child in element.child_elements_iter() {
                child.remove_attribute(name);
            }
        }
        for value in common_attributes.into_values() {
            let Attr::Transform(Inheritable::Defined(value)) = value else {
                element.set_attribute(value);
                continue;
            };

            if let Some(mut attr) =
                get_attribute_mut!(element, Transform).and_then(inheritable::map_ref_mut)
            {
                attr.0.extend(value.0);
            } else {
                element.set_attribute(Attr::Transform(Inheritable::Defined(value)));
            }
        }
        Ok(())
    }
}

fn get_common_attributes<'input>(
    parent: &Element<'input, '_>,
) -> BTreeMap<AttrId<'input>, Attr<'input>> {
    let mut common_attributes: BTreeMap<_, _> = parent
        .first_element_child()
        .expect("element should have >1 child")
        .attributes()
        .into_iter()
        .filter(|a| {
            is_attribute!(a, Transform) || a.name().info().contains(AttributeInfo::Inheritable)
        })
        .map(|a| (a.name().clone(), a.clone()))
        .collect();
    parent.child_elements_iter().for_each(|e| {
        let attrs = e.attributes();
        common_attributes
            .retain(|name, value| attrs.get_named_item(name).is_some_and(|a| &*a == value));
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
          <ellipse fill="red" color="#000" />
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
