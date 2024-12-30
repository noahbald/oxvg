use std::cell::RefCell;

use oxvg_ast::{
    atom::Atom,
    document::Document,
    element::Element,
    node::{self, Node},
    visitor::{PrepareOutcome, Visitor},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use super::ContextFlags;

#[derive(OptionalDefault)]
pub struct MergeStyles {
    enabled: bool,
    first_style: RefCell<Option<RefCell<Box<dyn node::Ref>>>>,
    is_cdata: RefCell<bool>,
}

impl<E: Element> Visitor<E> for MergeStyles {
    type Error = String;

    fn prepare(
        &mut self,
        _document: &E::ParentChild,
        _context_flags: &ContextFlags,
    ) -> PrepareOutcome {
        if self.enabled {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, context: &super::Context<E>) -> Result<(), String> {
        if element.prefix().is_none() && element.local_name() != "style".into() {
            return Ok(());
        }

        if let Some(style_type) = element
            .get_attribute(&"type".into())
            .as_ref()
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
                self.is_cdata.replace_with(|_| true);
            }
        });
        let css = css.trim();
        if css.is_empty() {
            log::debug!("Removed empty style");
            element.remove();
            return Ok(());
        }

        let media_name = &"media".into();
        let css = if let Some(media) = element.get_attribute(media_name) {
            element.remove_attribute(media_name);
            format!("@media {media}{{{css}}}")
        } else {
            css.to_string()
        };

        let first_style = self.first_style.borrow();
        if let Some(first_style) = &*first_style {
            let node = &mut *first_style.borrow_mut();
            let mut node = node
                .inner_as_any()
                .downcast_ref::<E::Child>()
                .unwrap()
                .clone();
            node.append_child(node.text(css.into()));
            element.remove();
            log::debug!("Merged style");
        } else {
            drop(first_style);
            element.clone().set_text_content(css.into());
            self.first_style
                .replace_with(|_| Some(RefCell::new(element.as_ref())));
            log::debug!("Assigned first style");
        }
        Ok(())
    }

    fn exit_document(&mut self, document: &mut E) -> Result<(), String> {
        if !&*self.is_cdata.borrow() {
            return Ok(());
        }

        let Some(style) = &*self.first_style.borrow() else {
            return Ok(());
        };
        let style = &mut *style.borrow_mut();
        let mut style = style
            .inner_as_any()
            .downcast_ref::<E::ParentChild>()
            .unwrap()
            .clone();
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

impl Default for MergeStyles {
    fn default() -> Self {
        Self {
            enabled: true,
            first_style: RefCell::new(None),
            is_cdata: RefCell::new(false),
        }
    }
}

impl Clone for MergeStyles {
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            first_style: match &*self.first_style.borrow() {
                Some(node) => {
                    let node = node.borrow().clone();
                    RefCell::new(Some(RefCell::new(node)))
                }
                None => RefCell::new(None),
            },
            is_cdata: self.is_cdata.clone(),
        }
    }
}

impl<'de> Deserialize<'de> for MergeStyles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let enabled = bool::deserialize(deserializer)?;
        Ok(Self {
            enabled,
            first_style: RefCell::new(None),
            is_cdata: RefCell::new(false),
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
