use lightningcss::{
    declaration::DeclarationBlock,
    properties::{
        custom::{CustomProperty, Token, TokenOrValue},
        Property,
    },
    values::percentage::DimensionPercentage,
    visit_types,
    visitor::{Visit, VisitTypes},
};
use oxvg_ast::{
    attribute::data::{
        inheritable::Inheritable,
        presentation::{EnableBackground, LengthPercentage},
    },
    element::{data::ElementId, Element},
    get_attribute, get_attribute_mut, remove_attribute,
    visitor::{Context, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

use crate::error::JobsError;

use super::ContextFlags;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
/// Cleans up `enable-background` attributes and styles. It will only remove it if
/// - The document has no `<filter>` element; and
/// - The value matches the document's width and height; or
/// - Replace `new` when it matches the width and height of a `<mask>` or `<pattern>`
///
/// This job will:
/// - Drop `enable-background` on `<svg>` node, if it matches the node's width and height
/// - Set `enable-background` to `"new"` on `<mask>` or `<pattern>` nodes, if it matches the
///   node's width and height
///
/// # Correctness
///
/// This attribute is deprecated and won't visually affect documents in most renderers. For outdated
/// renderers that still support it, there may be a visual change.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct CleanupEnableBackground(pub bool);

struct State {
    contains_filter: bool,
}

struct EnableBackgroundDimensions<'a> {
    width: &'a str,
    height: &'a str,
}

impl<'input, 'arena> Visitor<'input, 'arena> for CleanupEnableBackground {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        info: &Info<'input, 'arena>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        if !self.0 {
            return Ok(PrepareOutcome::skip);
        }
        if let Some(root) = document.find_element() {
            State::new(&root).start(&mut document.clone(), info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

struct EnableBackgroundVisitor;
impl<'input> lightningcss::visitor::Visitor<'input> for EnableBackgroundVisitor {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(PROPERTIES)
    }
    fn visit_declaration_block(
        &mut self,
        decls: &mut DeclarationBlock<'input>,
    ) -> Result<(), Self::Error> {
        let first_token = TokenOrValue::Token(Token::Ident("new".into()));
        let remove_enable_background_new = |property: &Property| match property {
            Property::Custom(CustomProperty { name, value }) => {
                name.as_ref() != "enable-background"
                    || value.0.first() != Some(&first_token)
                    || todo!("validate this")
            }
            _ => true,
        };
        decls
            .important_declarations
            .retain(remove_enable_background_new);
        decls.declarations.retain(remove_enable_background_new);
        Ok(())
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let style = get_attribute_mut!(element, Style);
        if let Some(mut style) = style {
            style.0.visit(&mut EnableBackgroundVisitor)?;
            if style.is_empty() {
                drop(style);
                remove_attribute!(element, Style);
            }
        }

        if !self.contains_filter {
            remove_attribute!(element, EnableBackground);
            return Ok(());
        }

        let mut enable_background = get_attribute_mut!(element, EnableBackground);
        let Some(Inheritable::Defined(enable_background)) = enable_background.as_deref_mut() else {
            return Ok(());
        };

        if !enabled_background_matches(element, &*enable_background) {
            return Ok(());
        }
        match element.qual_name().unaliased() {
            ElementId::Svg => {
                remove_attribute!(element, EnableBackground);
            }
            ElementId::Mask | ElementId::Pattern => {
                *enable_background = EnableBackground::New(None);
            }
            _ => {}
        }
        Ok(())
    }
}

impl State {
    fn new<'input, 'arena>(root: &Element<'input, 'arena>) -> Self {
        Self {
            contains_filter: root
                .breadth_first()
                .any(|element| *element.qual_name() == ElementId::Filter),
        }
    }
}

fn enabled_background_matches<'input, 'arena>(
    element: &Element<'input, 'arena>,
    dimensions: &EnableBackground,
) -> bool {
    let EnableBackground::New(Some((_x, _y, eb_width, eb_height))) = dimensions else {
        return false;
    };
    let width = get_attribute!(element, Width);
    let Some(LengthPercentage(DimensionPercentage::Dimension(width))) = width.as_deref() else {
        return false;
    };
    let height = get_attribute!(element, Height);
    let Some(LengthPercentage(DimensionPercentage::Dimension(height))) = height.as_deref() else {
        return false;
    };
    width.to_px().is_some_and(|px| px == *eb_width)
        && height.to_px().is_some_and(|px| px == *eb_height)
}

impl Default for CleanupEnableBackground {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn cleanup_enable_background() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupEnableBackground": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100.5" height=".5" enable-background="new 0 0 100.5 .5">
    <!-- Remove svg's enable-background on matching size -->
    <defs>
        <filter id="ShiftBGAndBlur">
            <feOffset dx="0" dy="75"/>
        </filter>
    </defs>
    test
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupEnableBackground": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50" enable-background="new 0 0 100 50">
    <!-- Keep svg's enable-background on mis-matching size -->
    <defs>
        <filter id="ShiftBGAndBlur">
            <feOffset dx="0" dy="75"/>
        </filter>
    </defs>
    test
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupEnableBackground": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Replace matching mask or pattern's enable-background with "new" -->
    <defs>
        <filter id="ShiftBGAndBlur">
            <feOffset dx="0" dy="75"/>
        </filter>
    </defs>
    <mask width="100" height="50" enable-background="new 0 0 100 50">
        test
    </mask>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupEnableBackground": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Remove enable-background when no filter is present -->
    <mask width="100" height="50" enable-background="new 0 0 100 50">
        test
    </mask>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        // TODO: Should apply to inline styles as well, removing the style attribute if it all
        // declarations are removed.
        r#"{ "cleanupEnableBackground": true }"#,
        Some(
            r##"<svg height="100" width="100" style="enable-background:new 0 0 100 100">
  <circle cx="50" cy="50" r="40" stroke="#000" stroke-width="3" fill="red"/>
</svg>"##
        )
    )?);

    Ok(())
}
