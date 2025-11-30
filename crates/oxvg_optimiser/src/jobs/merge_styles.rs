use std::cell::{Cell, RefCell};

use lightningcss::rules::{media::MediaRule, CssRule, CssRuleList, Location};
use oxvg_ast::{
    element::Element,
    get_attribute, is_element,
    node::{self, Node},
    remove_attribute,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::attribute::uncategorised::MediaQueryList;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::JobsError;

use super::ContextFlags;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[derive(Debug)]
struct State<'input, 'arena> {
    first_style: RefCell<Option<Element<'input, 'arena>>>,
    is_cdata: Cell<bool>,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(transparent))]
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
pub struct MergeStyles(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for MergeStyles {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State::default().start(&mut document.clone(), context.info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'input, 'arena> {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !is_element!(element, Style) {
            return Ok(());
        }

        if let Some(style_type) = get_attribute!(element, TypeStyle) {
            if !style_type.is_empty() && &**style_type != "text/css" {
                log::debug!("Not merging style: unsupported type");
                return Ok(());
            }
        }

        if context.flags.contains(ContextFlags::within_foreign_object) {
            log::debug!("Not merging style: foreign-object");
            return Ok(());
        }

        let mut css = Vec::new();
        element.child_nodes_iter().for_each(|node| {
            if let Some(style) = node.style() {
                css.extend(style.borrow().0.clone());
            }
            if node.node_type() == node::Type::CDataSection {
                self.is_cdata.set(true);
            }
        });
        if css.is_empty() {
            log::debug!("Removed empty style");
            element.remove();
            return Ok(());
        }

        if let Some(MediaQueryList(query)) = remove_attribute!(element, Media) {
            css = vec![CssRule::Media(MediaRule {
                query,
                rules: CssRuleList(css),
                loc: Location {
                    source_index: 0,
                    line: 0,
                    column: 0,
                },
            })];
        }

        let first_style = self.first_style.borrow();
        if let Some(node) = &*first_style {
            if let Some(style) = node.style() {
                style.borrow_mut().0.extend(css);
            } else {
                unreachable!("Style node should have been set");
            }
            element.remove();
            log::debug!("Merged style");
        } else {
            drop(first_style);
            element.set_style_content(CssRuleList(css), &context.info.allocator);
            self.first_style.replace(Some(element.clone()));
            log::debug!("Assigned first style");
        }
        Ok(())
    }

    fn exit_document(
        &self,
        document: &Element<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), JobsError<'input>> {
        if !self.is_cdata.get() {
            return Ok(());
        }

        let Some(style) = &mut *self.first_style.borrow_mut() else {
            return Ok(());
        };
        let Some(css) = style.style() else {
            style.remove();
            return Ok(());
        };
        style.child_nodes_iter().for_each(Node::remove);
        let child = document
            .as_document()
            .create_style_node(css.replace(CssRuleList(vec![])), &context.info.allocator);
        style.append_child(child);
        Ok(())
    }
}

impl Default for MergeStyles {
    fn default() -> Self {
        Self(true)
    }
}

impl Default for State<'_, '_> {
    fn default() -> Self {
        Self {
            first_style: RefCell::new(None),
            is_cdata: Cell::new(false),
        }
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
    <!-- Should remove empty styles -->
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

    // WARN: CData not supported by implementations
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
