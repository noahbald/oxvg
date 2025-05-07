use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node::{self, Node},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::EVENT_ATTRS;
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(transparent)]
/// Removes `<script>` elements, event attributes, and javascript `href`s from the document.
///
/// This can help remove the risk of Cross-site scripting (XSS) attacks.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// It's likely to break interactive documents.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveScripts(pub bool);

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveScripts {
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
        if element.prefix().is_none() && element.local_name().as_ref() == "script" {
            log::debug!("removing script");
            element.remove();
            return Ok(());
        }

        element
            .attributes()
            .retain(|attr| !EVENT_ATTRS.contains(&attr.name().formatter().to_string()));

        Ok(())
    }

    fn exit_element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if element.prefix().is_some() && element.local_name().as_ref() != "a" {
            return Ok(());
        }

        let is_href_js = element.attributes().into_iter().any(|attr| {
            if attr.local_name().as_ref() != "href" {
                return false;
            }

            if !attr.value().trim_start().starts_with("javascript:") {
                return false;
            }
            true
        });
        if !is_href_js {
            return Ok(());
        }

        element.retain_children(|node| node.node_type() != node::Type::Text);
        element.flatten();

        Ok(())
    }
}

#[test]
fn remove_scripts() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeScripts": true }"#,
        Some(
            r#"<svg version="1.1" id="Layer_1" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" viewBox="0 0 100 100" style="enable-background:new 0 0 100 100;" xml:space="preserve">
    <script></script>
    <circle class="st0" cx="50" cy="50" r="50" />
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeScripts": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <a href="javascript:(() => { alert('uwu') })();">
    <text y="10" onclick="alert('uwu')">uwu</text>
  </a>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeScripts": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <a href="https://yewtu.be/watch?v=dQw4w9WgXcQ">
    <text y="10" onclick="alert('uwu')">uwu</text>
  </a>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeScripts": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" version="1.1">
  <script>alert('uwu')</script>
  <g onclick="alert('uwu')">
    <text y="10">uwu</text>
  </g>
  <a href="javascript:(() => { alert('uwu') })();">
    <text y="20">uwu</text>
  </a>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeScripts": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:uwu="http://www.w3.org/1999/xlink" viewBox="0 0 100 100" version="1.1">
  <a href="javascript:(() => { alert('uwu') })();">
    <text y="20">uwu</text>
  </a>
  <a uwu:href="javascript:(() => { alert('uwu') })();">
    <text y="30">uwu</text>
  </a>
</svg>"#
        ),
    )?);

    Ok(())
}
