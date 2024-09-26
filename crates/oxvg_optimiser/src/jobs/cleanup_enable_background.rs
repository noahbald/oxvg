use std::rc::Rc;

use markup5ever::local_name;
use oxvg_selectors::Element;
use regex::Regex;
use serde::Deserialize;

use crate::{Context, Job, PrepareOutcome};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupEnableBackground {
    #[serde(skip_deserializing)]
    contains_filter: bool,
}

struct EnableBackgroundDimensions<'a> {
    width: &'a str,
    height: &'a str,
}

impl Job for CleanupEnableBackground {
    fn prepare(&mut self, document: &rcdom::RcDom) -> PrepareOutcome {
        let Some(root) = &Element::from_document_root(document) else {
            return PrepareOutcome::None;
        };
        self.prepare_contains_filter(root);
        PrepareOutcome::None
    }

    /// Cleans up `enable-background`, unless document uses `<filter>` elements.
    ///
    /// Only cleans up attribute values
    /// TODO: Clean up inline-styles
    ///
    /// This job will:
    /// - Drop `enable-background` on `<svg>` node, if it matches the node's width and height
    /// - Set `enable-background` to `"new"` on `<mask>` or `<pattern>` nodes, if it matches the
    /// node's width and height
    fn run(&self, node: &Rc<rcdom::Node>, _context: &Context) {
        let element = oxvg_selectors::Element::new(node.clone());

        if let Some(mut style) = element.get_attr(&local_name!("style")) {
            style.value = Regex::new(r"(^|;)\s*enable-background\s*:\s*new[\d\s]*")
                .unwrap()
                .replace_all(style.value.as_ref(), "")
                .to_string()
                .into();
        }

        if !self.contains_filter {
            element.remove_attr(&local_name!("enable-background"));
            return;
        };

        let Some(enable_background) = element.get_attr(&local_name!("enable-background")) else {
            return;
        };
        let Some(name) = element.get_name() else {
            return;
        };

        let enabled_background_dimensions =
            Self::get_enabled_background_dimensions(&enable_background);
        let matches_dimensions =
            Self::enabled_background_matches(&element, enabled_background_dimensions);
        if matches_dimensions && name == local_name!("svg") {
            element.remove_attr(&local_name!("enable-background"));
        } else if matches_dimensions
            && (name == local_name!("mask") || name == local_name!("pattern"))
        {
            element.set_attr(&local_name!("enable-background"), "new".into());
        }
    }
}

impl CleanupEnableBackground {
    fn prepare_contains_filter(&mut self, root: &Element) {
        self.contains_filter = root.select("filter").unwrap().next().is_some();
    }

    fn get_enabled_background_dimensions(
        attr: &markup5ever::Attribute,
    ) -> Option<EnableBackgroundDimensions> {
        let parameters: Vec<_> = attr.value.split_whitespace().collect();
        // Only allow `new <x> <y> <width> <height>`
        if parameters.len() != 5 {
            return None;
        }

        Some(EnableBackgroundDimensions {
            width: parameters.get(3)?,
            height: parameters.get(4)?,
        })
    }

    fn enabled_background_matches(
        element: &Element,
        dimensions: Option<EnableBackgroundDimensions>,
    ) -> bool {
        use markup5ever::tendril::Tendril;

        let Some(dimensions) = dimensions else {
            return false;
        };
        let Some(width) = element.get_attr(&local_name!("width")) else {
            return false;
        };
        let Some(height) = element.get_attr(&local_name!("height")) else {
            return false;
        };
        Tendril::from(dimensions.width) == width.value
            && Tendril::from(dimensions.height) == height.value
    }
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
