use oxvg_ast::{
    attribute::Attr,
    element::Element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::ContextFlags;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupEnableBackground {
    #[serde(skip_deserializing, skip_serializing)]
    contains_filter: bool,
}

struct EnableBackgroundDimensions<'a> {
    width: &'a str,
    height: &'a str,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for CleanupEnableBackground {
    type Error = String;

    fn prepare(&mut self, document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        let Some(root) = document.find_element() else {
            return PrepareOutcome::none;
        };
        self.prepare_contains_filter(&root);
        PrepareOutcome::none
    }

    /// Cleans up `enable-background`, unless document uses `<filter>` elements.
    ///
    /// Only cleans up attribute values
    /// TODO: Clean up inline-styles
    ///
    /// This job will:
    /// - Drop `enable-background` on `<svg>` node, if it matches the node's width and height
    /// - Set `enable-background` to `"new"` on `<mask>` or `<pattern>` nodes, if it matches the
    ///   node's width and height
    fn element(
        &mut self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let style_name = &"style".into();
        if let Some(mut style) = element.get_attribute_node_local_mut(style_name) {
            let new_value = ENABLE_BACKGROUND
                .replace_all(style.value().as_ref(), "")
                .to_string();
            if new_value.is_empty() {
                drop(style);
                element.remove_attribute_local(style_name);
            } else {
                style.set_value(new_value.into());
            }
        }

        let enable_background_localname = "enable-background".into();
        if !self.contains_filter {
            element.remove_attribute_local(&enable_background_localname);
            return Ok(());
        };

        let Some(enable_background) = element.get_attribute_local(&"enable-background".into())
        else {
            return Ok(());
        };
        let name = element.local_name();

        let enabled_background_dimensions =
            Self::get_enabled_background_dimensions(enable_background.as_ref());
        let matches_dimensions =
            Self::enabled_background_matches(element, enabled_background_dimensions);
        drop(enable_background);
        if matches_dimensions && name.as_ref() == "svg" {
            element.remove_attribute_local(&enable_background_localname);
        } else if matches_dimensions && (name.as_ref() == "mask" || name.as_ref() == "pattern") {
            element.set_attribute_local(enable_background_localname, "new".into());
        }
        Ok(())
    }
}

impl CleanupEnableBackground {
    fn prepare_contains_filter<'arena, E: Element<'arena>>(&mut self, root: &E) {
        self.contains_filter = root.select("filter").unwrap().next().is_some();
    }

    fn get_enabled_background_dimensions(attr: &str) -> Option<EnableBackgroundDimensions> {
        let parameters: Vec<_> = attr.split_whitespace().collect();
        // Only allow `new <x> <y> <width> <height>`
        if parameters.len() != 5 {
            return None;
        }

        Some(EnableBackgroundDimensions {
            width: parameters.get(3)?,
            height: parameters.get(4)?,
        })
    }

    fn enabled_background_matches<'arena, E: Element<'arena>>(
        element: &E,
        dimensions: Option<EnableBackgroundDimensions>,
    ) -> bool {
        let Some(dimensions) = dimensions else {
            return false;
        };
        let Some(width) = element.get_attribute_local(&"width".into()) else {
            return false;
        };
        let Some(height) = element.get_attribute_local(&"height".into()) else {
            return false;
        };
        dimensions.width == width.as_ref() && dimensions.height == height.as_ref()
    }
}

lazy_static! {
    static ref ENABLE_BACKGROUND: Regex =
        Regex::new(r"(^|;)\s*enable-background\s*:\s*new[\d\s]*").unwrap();
}

#[test]
fn cleanup_enable_background() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupEnableBackground": {} }"#,
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
        r#"{ "cleanupEnableBackground": {} }"#,
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
        r#"{ "cleanupEnableBackground": {} }"#,
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
        r#"{ "cleanupEnableBackground": {} }"#,
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
        r#"{ "cleanupEnableBackground": {} }"#,
        Some(
            r##"<svg height="100" width="100" style="enable-background:new 0 0 100 100">
  <circle cx="50" cy="50" r="40" stroke="#000" stroke-width="3" fill="red"/>
</svg>"##
        )
    )?);

    Ok(())
}
