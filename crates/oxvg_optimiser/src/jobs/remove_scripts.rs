use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node::{self, Node},
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::EVENT_ATTRS;
use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
pub struct RemoveScripts(bool);

impl<E: Element> Visitor<E> for RemoveScripts {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        if element.prefix().is_none() && element.local_name().as_ref() == "script" {
            element.remove();
            return Ok(());
        }

        element
            .attributes()
            .retain(|attr| !EVENT_ATTRS.contains(&attr.name().formatter().to_string()));

        Ok(())
    }

    fn exit_element(
        &mut self,
        element: &mut E,
        _context: &mut Context<E>,
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
            r#"<?xml version="1.0" encoding="utf-16"?>
<svg version="1.1" id="Layer_1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" x="0px" y="0px" viewBox="0 0 100 100" style="enable-background:new 0 0 100 100;" xml:space="preserve">
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
