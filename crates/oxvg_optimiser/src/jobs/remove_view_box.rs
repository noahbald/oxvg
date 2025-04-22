use oxvg_ast::{
    element::Element,
    name::Name,
    node::{self, Node},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemoveViewBox(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveViewBox {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let name = element.qual_name();
        if name.prefix().is_some() {
            return Ok(());
        }

        match name.local_name().as_ref() {
            "pattern" | "svg" | "symbol" => {}
            _ => return Ok(()),
        };

        let view_box_name = "viewBox".into();
        let Some(view_box_atom) = element.get_attribute_local(&view_box_name) else {
            return Ok(());
        };
        let view_box = view_box_atom.as_ref();
        let Some(width_atom) = element.get_attribute_local(&"width".into()) else {
            return Ok(());
        };
        let width = width_atom.as_ref();
        let Some(height_atom) = element.get_attribute_local(&"height".into()) else {
            return Ok(());
        };
        let height = height_atom.as_ref();

        if name.local_name().as_ref() == "svg"
            && element
                .parent_node()
                .is_some_and(|n| n.node_type() != node::Type::Document)
        {
            // TODO: remove width/height for such case instead
            log::debug!("not removing viewbox from root svg");
            return Ok(());
        }

        let mut nums = Vec::with_capacity(4);
        nums.extend(SEPARATOR.split(view_box));
        if nums.len() != 4 {
            return Ok(());
        }

        if nums[0] == "0"
            && nums[1] == "0"
            && width.strip_suffix("px").unwrap_or(width) == nums[2]
            && height.strip_suffix("px").unwrap_or(height) == nums[3]
        {
            drop(view_box_atom);
            drop(width_atom);
            drop(height_atom);
            log::debug!("removing viewBox from element");
            element.remove_attribute_local(&view_box_name);
        }

        Ok(())
    }
}

impl Default for RemoveViewBox {
    fn default() -> Self {
        Self(true)
    }
}

lazy_static! {
    pub static ref SEPARATOR: regex::Regex = regex::Regex::new(r"[ ,]+").unwrap();
}

#[test]
fn remove_view_box() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100.5" height=".5" viewBox="0 0 100.5 .5">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50" viewBox="0 0 100 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0, 0, 100, 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50" viewBox="-25 -25 50 50">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeViewBox": true }"#,
        Some(
            r##"<svg width="480" height="360" viewBox="0 0 480 360" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <defs>
    <svg id="svg-sub-root" viewBox="0 0 450 450" width="450" height="450">
      <rect x="225" y="0" width="220" height="220" style="fill:magenta"/>
      <rect x="0" y="225" width="220" height="220" style="fill:#f0f"/>
      <rect x="225" y="225" width="220" height="220" fill="#f0f"/>
    </svg>
  </defs>
  <use x="60" y="50" width="240" height="240" xlink:href="#svg-sub-root"/>
  <rect x="300" y="170" width="118" height="118" fill="magenta"/>
</svg>"##
        ),
    )?);

    Ok(())
}
