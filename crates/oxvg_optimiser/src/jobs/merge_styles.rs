use oxvg_ast::{
    atom::Atom,
    document::Document,
    element::Element,
    node::{self, Node},
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::Deserialize;

use super::ContextFlags;

pub struct MergeStyles<E: Element> {
    enabled: bool,
    first_style: Option<E>,
    is_cdata: bool,
}

impl<E: Element> Visitor<E> for MergeStyles<E> {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.enabled {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, context: &mut Context<E>) -> Result<(), String> {
        if element.prefix().is_none() && element.local_name().as_str() != "style" {
            return Ok(());
        }

        if let Some(style_type) = element
            .get_attribute_local(&"type".into())
            .as_deref()
            .map(Atom::as_str)
        {
            if !style_type.is_empty() && style_type != "text/css" {
                log::debug!("Not merging style: unsupported type");
                return Ok(());
            }
        }

        if context.flags.contains(ContextFlags::within_foreign_object) {
            log::debug!("Not merging style: foreign-object");
            return Ok(());
        }

        let mut css = String::new();
        element.for_each_child(|node| {
            if let Some(text) = node.text_content() {
                css.push_str(&text);
            }
            if node.node_type() == node::Type::CDataSection {
                self.is_cdata = true;
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

        if let Some(node) = &self.first_style {
            let mut node = node.clone();
            node.append_child(node.text(css.into()));
            element.remove();
            log::debug!("Merged style");
        } else {
            element.clone().set_text_content(css.into());
            self.first_style = Some(element.clone());
            log::debug!("Assigned first style");
        }
        Ok(())
    }

    fn exit_document(&mut self, document: &mut E, _context: &Context<E>) -> Result<(), String> {
        if !self.is_cdata {
            return Ok(());
        }

        let Some(style) = self.first_style.as_mut() else {
            return Ok(());
        };
        let Some(text) = style.text_content() else {
            style.remove();
            return Ok(());
        };
        style.for_each_child(|child| child.remove());
        let c_data = document.as_document().create_c_data_section(text.into());
        style.append_child(c_data);
        Ok(())
    }
}

impl<E: Element> Default for MergeStyles<E> {
    fn default() -> Self {
        Self {
            enabled: true,
            first_style: None,
            is_cdata: false,
        }
    }
}

impl<E: Element> Clone for MergeStyles<E> {
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            first_style: self.first_style.clone(),
            is_cdata: self.is_cdata,
        }
    }
}

impl<'de, E: Element> Deserialize<'de> for MergeStyles<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let enabled = bool::deserialize(deserializer)?;
        Ok(Self {
            enabled,
            first_style: None,
            is_cdata: false,
        })
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
