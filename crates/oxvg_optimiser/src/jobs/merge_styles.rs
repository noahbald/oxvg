use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
};

use derive_where::derive_where;
use oxvg_ast::{
    atom::Atom,
    document::Document,
    element::Element,
    node::{self, Node},
    visitor::{Context, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

use super::ContextFlags;

#[derive_where(Debug)]
struct State<'arena, E: Element<'arena>> {
    first_style: RefCell<Option<E>>,
    is_cdata: Cell<bool>,
    marker: PhantomData<&'arena ()>,
}

#[derive(Debug, Clone)]
/// Merge multiple `<style>` elements into one
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
pub struct MergeStyles(bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for MergeStyles {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State::default().start(&mut document.clone(), info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'arena, E> {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        if element.prefix().is_none() && element.local_name().as_str() != "style" {
            return Ok(());
        }

        if let Some(style_type) = element.get_attribute_local(&"type".into()) {
            if !style_type.is_empty() && style_type.as_ref() != "text/css" {
                log::debug!("Not merging style: unsupported type");
                return Ok(());
            }
        }

        if context.flags.contains(ContextFlags::within_foreign_object) {
            log::debug!("Not merging style: foreign-object");
            return Ok(());
        }

        let mut css = String::new();
        element.child_nodes_iter().for_each(|node| {
            if let Some(text) = node.text_content() {
                css.push_str(&text);
            }
            if node.node_type() == node::Type::CDataSection {
                self.is_cdata.set(true);
            }
        });
        let css = css.trim();
        if css.is_empty() {
            log::debug!("Removed empty style");
            element.remove();
            return Ok(());
        }

        let media_name = &"media".into();
        let css = if let Some(media) = element.get_attribute_local(media_name) {
            let css = format!("@media {}{{{css}}}", media.as_ref());
            drop(media);
            element.remove_attribute_local(media_name);
            css
        } else {
            css.to_string()
        };

        let first_style = self.first_style.borrow();
        if let Some(node) = &*first_style {
            node.append_child(node.text(css.into(), &context.info.arena));
            element.remove();
            log::debug!("Merged style");
        } else {
            drop(first_style);
            element
                .clone()
                .set_text_content(css.into(), &context.info.arena);
            self.first_style.replace(Some(element.clone()));
            log::debug!("Assigned first style");
        }
        Ok(())
    }

    fn exit_document(
        &self,
        document: &mut E,
        context: &Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        if !self.is_cdata.get() {
            return Ok(());
        }

        let Some(style) = &mut *self.first_style.borrow_mut() else {
            return Ok(());
        };
        let Some(text) = style.text_content() else {
            style.remove();
            return Ok(());
        };
        style.child_nodes_iter().for_each(|child| child.remove());
        let c_data = document
            .as_document()
            .create_c_data_section(text, &context.info.arena);
        style.append_child(c_data);
        Ok(())
    }
}

impl Default for MergeStyles {
    fn default() -> Self {
        Self(true)
    }
}

impl<'arena, E: Element<'arena>> Default for State<'arena, E> {
    fn default() -> Self {
        Self {
            first_style: RefCell::new(None),
            is_cdata: Cell::new(false),
            marker: PhantomData,
        }
    }
}

impl<'de> Deserialize<'de> for MergeStyles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let enabled = bool::deserialize(deserializer)?;
        Ok(Self(enabled))
    }
}

impl Serialize for MergeStyles {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn merge_styles() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- No changes needed when there's only one style element -->
    <style>
        .st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; }
    </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Appends media query to style -->
    <style>.st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; }</style>
    <style>
        @media screen and (max-width: 200px) { .st0 { display: none; } }
    </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should have media attribute -->
    <style media="print">.st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; }</style>
    <style>.test { background: red; }</style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should handle multiple media attributes -->
    <style media="print">.st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; }</style>
    <style>.test { background: red; }</style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
    <style media="only screen and (min-width: 600px)">.wrapper { color: blue; }</style>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Shouldn't affect style-less documents -->
    <rect width="100" height="100" class="st0"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should remove empty styles -->
    <style></style>
    <style>
        .st0{ fill:red; padding-top: 1em; padding-right: 1em; padding-bottom: 1em; padding-left: 1em; }
    </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should remove empty styles -->
    <style></style>
    <style>
    </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should remove empty styles -->
    <style></style>
    <style></style>
    <style>
        .test { color: red; }
    </style>
    <style></style>
    <style></style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should handle type attribute -->
    <style>
        .a { fill: blue; }
    </style>
    <style type="">
        .b { fill: green; }
    </style>
        <style type="text/css">
        .c { fill: red; }
    </style>
    <style type="text/invalid">
        .d { fill: blue; }
    </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should remove empty syles -->
    <style>
	  </style>
    <rect width="100" height="100" class="st0" style="stroke-width:3;margin-top:1em;margin-right:1em;margin-bottom:1em;margin-left:1em"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergeStyles": true }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Skip styles inside foreignObject -->
    <foreignObject>
        <style>
        .st0 { fill: yellow; }
        </style>
    </foreignObject>
    <style>
        .st1 { fill: red; }
    </style>
</svg>"#
        ),
    )?);

    // WARN: CData not supported by rcdom implementation
    // insta::assert_snapshot!(test_config(
    //     r#"{ "mergeStyles": true }"#,
    //     Some(
    //         r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    //     <style>
    //         .st0 { fill: yellow; }
    //     </style>
    //     <style>
    //         <![CDATA[
    //             .st1 { fill: red; }
    //         ]]>
    //     </style>
    // </svg>"#
    //     ),
    // )?);

    Ok(())
}
