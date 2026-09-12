use oxvg_ast::{
    element::Element,
    is_element, node,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::attribute::{Attr, AttributeGroup};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::{error::JobsError, utils::is_executable_url::is_exectable_url};

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", serde(transparent))]
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

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveScripts {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if is_element!(element, Script) {
            log::debug!("removing script");
            element.remove();
            return Ok(());
        }

        element.attributes().retain(|attr| {
            !attr
                .name()
                .attribute_group()
                .intersects(AttributeGroup::event())
        });

        Ok(())
    }

    fn exit_element(
        &self,
        element: Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !is_element!(element, A) {
            return Ok(());
        }

        let is_href_js = element.attributes().into_iter().any(|attr| {
            let (Attr::Href(href) | Attr::XLinkHref(href)) = attr.unaliased() else {
                return false;
            };
            is_exectable_url(href)
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

    insta::assert_snapshot!(test_config(
        r#"{ "removeScripts": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:uwu="http://www.w3.org/1999/xlink" viewBox="0 0 100 100">
  <a href="data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==">
    <text y="10">HTML</text>
  </a>
  <a uwu:href="DATA:application/xhtml+xml;charset=utf-8,%3Cscript%3Ealert(1)%3C/script%3E">
    <text y="20">XHTML</text>
  </a>
  <a href="data:image/svg+xml;base64,PHN2ZyBvbmxvYWQ9ImFsZXJ0KDEpIi8+">
    <text y="30">SVG</text>
  </a>
  <a href="data:image/png;base64,iVBORw0KGgo=">
    <text y="40">PNG</text>
  </a>
  <a href="vbscript:msgbox(1)">
    <text y="50">VBScript</text>
  </a>
</svg>"#
        ),
    )?);

    Ok(())
}
