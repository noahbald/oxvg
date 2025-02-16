use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    collections::{PATH_ELEMS, REFERENCES_PROPS},
    regex::REFERENCES_URL,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MoveGroupAttrsToElems(bool);

impl<E: Element> Visitor<E> for MoveGroupAttrsToElems {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), Self::Error> {
        let name = element.qual_name();
        if name.prefix().is_some() || name.local_name().as_ref() != "g" {
            return Ok(());
        }
        if element.is_empty() {
            return Ok(());
        }
        let transform_name = "transform".into();
        let Some(transform) = element.get_attribute_local(&transform_name) else {
            return Ok(());
        };
        if element.attributes().into_iter().any(|a| {
            let name = a.name().formatter().to_string();
            let value = a.value();
            REFERENCES_PROPS.contains(&name) && REFERENCES_URL.is_match(value.as_ref())
        }) {
            return Ok(());
        }
        let id_name = &"id".into();
        if element.any_child(|n| {
            let Some(e) = E::new(n) else {
                return false;
            };
            let name = e.qual_name().formatter().to_string();
            !(PATH_ELEMS.contains(&name) || &name == "g" || &name == "text")
                || e.has_attribute_local(id_name)
        }) {
            return Ok(());
        }

        element.for_each_element_child(|e| match e.get_attribute_node_local_mut(&transform_name) {
            Some(mut child_attr) => {
                let value = format!("{} {}", transform.as_ref(), child_attr.value());
                child_attr.set_value(value.into());
            }
            None => e.set_attribute_local(transform_name.clone(), transform.clone()),
        });
        drop(transform);
        element.remove_attribute_local(&transform_name);

        Ok(())
    }
}

impl Default for MoveGroupAttrsToElems {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn move_group_attrs_to_elems() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "moveGroupAttrsToElems": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- append transform to children of `g` -->
    <g transform="scale(2)">
        <path transform="rotate(45)" d="M0,0 L10,20"/>
        <path transform="translate(10, 20)" d="M0,10 L20,30"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveGroupAttrsToElems": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- add transform to children of `g` -->
    <g transform="scale(2)">
        <path d="M0,0 L10,20"/>
        <path d="M0,10 L20,30"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveGroupAttrsToElems": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- move transform through multiple `g`s -->
    <g transform="rotate(30)">
        <g transform="scale(2)">
            <path d="M0,0 L10,20"/>
            <path d="M0,10 L20,30"/>
        </g>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveGroupAttrsToElems": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- move transform through multiple `g`s -->
    <g transform="rotate(30)">
        <g>
            <g transform="scale(2)">
                <path d="M0,0 L10,20"/>
                <path d="M0,10 L20,30"/>
            </g>
        </g>
        <path d="M0,10 L20,30"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveGroupAttrsToElems": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- don't move from group with reference -->
    <g transform="scale(2)" clip-path="url(#a)">
        <path d="M0,0 L10,20"/>
        <path d="M0,10 L20,30"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "moveGroupAttrsToElems": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- don't move for child with id -->
    <g transform="translate(0 -140)">
        <path id="c" transform="scale(.5)" d="M0,0 L10,20"/>
    </g>
    <use xlink:href="#c" transform="translate(-140)"/>
</svg>"##
        ),
    )?);

    Ok(())
}
