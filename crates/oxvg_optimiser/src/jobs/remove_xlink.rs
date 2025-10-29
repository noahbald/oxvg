use std::cell::RefCell;

use oxvg_ast::{
    atom::Atom,
    attribute::data::{uncategorised::Target, xlink::XLinkShow, Attr, AttrId},
    element::{category::ElementInfo, data::ElementId, Element},
    get_attribute,
    name::{Prefix, QualName, NS},
    remove_attribute, set_attribute,
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
/// Replaces `xlink` prefixed attributes to the native SVG equivalent.
///
/// # Correctness
///
/// This job may break compatibility with the SVG 1.1 spec.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct RemoveXlink {
    #[serde(default = "bool::default")]
    /// Whether to also convert xlink attributes for legacy elements which don't
    /// support the SVG 2 `href` attribute (e.g. `<cursor>`).
    ///
    /// This is safe to enable for SVGs to inline in HTML documents.
    pub include_legacy: bool,
}

struct State<'o, 'input> {
    options: &'o RemoveXlink,
    xlink_prefix_stack: RefCell<Vec<Atom<'input>>>,
    overridden_prefix_stack: RefCell<Vec<bool>>,
    /// Tracks when `xlink:href` is used in legacy element
    used_in_legacy_element_stack: RefCell<Vec<bool>>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for RemoveXlink {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        info: &Info<'input, 'arena>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        State {
            options: self,
            xlink_prefix_stack: RefCell::new(vec![]),
            overridden_prefix_stack: RefCell::new(vec![]),
            used_in_legacy_element_stack: RefCell::new(vec![]),
        }
        .start(&mut document.clone(), info, None)?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'_, 'input> {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut xlink_prefix_stack = self.xlink_prefix_stack.borrow_mut();
        let mut overridden_prefix_stack = self.overridden_prefix_stack.borrow_mut();
        let mut used_in_legacy_element_stack = self.used_in_legacy_element_stack.borrow_mut();
        for attr in element.attributes().into_iter() {
            let Attr::Unparsed {
                attr_id: AttrId::Unknown(QualName { prefix, local }),
                value,
            } = &*attr
            else {
                continue;
            };
            if *prefix != Prefix::XMLNS {
                continue;
            }
            if value == NS::XLink.uri() {
                xlink_prefix_stack.push(local.clone());
                overridden_prefix_stack.push(false);
                used_in_legacy_element_stack.push(false);
            } else if xlink_prefix_stack.last() == Some(local) {
                if let Some(last) = overridden_prefix_stack.last_mut() {
                    *last = true;
                }
            }
        }

        self.handle_show(element);
        self.handle_title(element, context);
        self.handle_href(element);

        Ok(())
    }

    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        element.attributes().retain(|attr| {
            let Attr::Unparsed {
                attr_id: AttrId::Unknown(QualName { prefix, .. }),
                value,
            } = attr
            else {
                return true;
            };
            if *prefix != Prefix::XMLNS {
                return true;
            }
            if value != NS::XLink.uri() {
                return true;
            }

            self.xlink_prefix_stack.borrow_mut().pop();
            let overridden_prefixes = self
                .overridden_prefix_stack
                .borrow_mut()
                .pop()
                .unwrap_or_else(|| false);
            let used_in_legacy_element = self
                .used_in_legacy_element_stack
                .borrow_mut()
                .pop()
                .unwrap_or_else(|| false);
            if !self.options.include_legacy && !overridden_prefixes && !used_in_legacy_element {
                return false;
            }
            true
        });

        Ok(())
    }
}

impl<'input> State<'_, 'input> {
    /// Replaces `xlink:show` with `target` when possible
    fn handle_show(&self, element: &Element<'input, '_>) {
        if element.has_attribute(&AttrId::Target) {
            element.remove_attribute(&AttrId::XLinkShow);
            return;
        }
        let Some(show) = get_attribute!(element, XLinkShow) else {
            return;
        };
        let target = match *show {
            XLinkShow::New => Some(Target::_Blank),
            XLinkShow::Replace => Some(Target::_Self),
            _ => None,
        };
        if let Some(target) = target {
            if target != Target::default() {
                set_attribute!(element, Target(target));
                remove_attribute!(element, XLinkShow);
            }
        }
    }

    fn handle_title<'arena>(
        &self,
        element: &Element<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) {
        if element
            .child_elements_iter()
            .any(|child| *child.qual_name() == ElementId::Title)
        {
            return;
        }
        let Some(title) = remove_attribute!(element, XLinkTitle) else {
            return;
        };

        let title_element = context
            .root
            .as_document()
            .create_element(ElementId::Title, &context.info.allocator);
        title_element.set_text_content(title, &context.info.allocator);
        element.insert(0, title_element.0);
    }

    fn handle_href(&self, element: &Element<'input, '_>) {
        let mut used_in_legacy_element = self.used_in_legacy_element_stack.borrow_mut();
        let used_in_legacy_element = used_in_legacy_element.last_mut();
        if element.has_attribute(&AttrId::Href) {
            return;
        }
        let Some(href) = get_attribute!(element, XLinkHref) else {
            return;
        };
        if !self.options.include_legacy && element.qual_name().info().contains(ElementInfo::Legacy)
        {
            if let Some(value) = used_in_legacy_element {
                *value = true;
            }
            return;
        }

        set_attribute!(element, Href(href.clone()));
    }
}

#[test]
fn remove_xlink() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeXlink": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 348.61 100">
    <!-- remove `xmlns:xlink` and replace `xlink:href` with `href` -->
    <defs>
        <linearGradient id="a" x1="263.36" y1="14.74" x2="333.47" y2="84.85" gradientUnits="userSpaceOnUse">
        <stop offset="0" stop-color="#45afe4"/>
        <stop offset="1" stop-color="#364f9e"/>
        </linearGradient>
        <linearGradient id="b" x1="262.64" y1="15.46" x2="332.75" y2="85.57" xlink:href="#a"/>
    </defs>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeXlink": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:uwu="http://www.w3.org/1999/xlink" viewBox="0 0 348.61 100">
  <!-- convert xlink aliased as uwu -->
  <defs>
    <linearGradient id="a" x1="263.36" y1="14.74" x2="333.47" y2="84.85" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#45afe4"/>
      <stop offset="1" stop-color="#364f9e"/>
    </linearGradient>
    <linearGradient id="b" x1="262.64" y1="15.46" x2="332.75" y2="85.57" uwu:href="#a"/>
  </defs>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeXlink": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 50 50">
  <!-- convert xlink:href, xlink:show, and xlink:title -->
  <a xlink:href="https://duckduckgo.com" xlink:show="new" xlink:title="DuckDuckGo Homepage">
    <text x="0" y="10">uwu</text>
  </a>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeXlink": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 50 50">
  <!-- drops other xlink attributes -->
  <defs>
    <linearGradient id="a" x1="263.36" y1="14.74" x2="333.47" y2="84.85" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#45afe4"/>
      <stop offset="1" stop-color="#364f9e"/>
    </linearGradient>
    <linearGradient id="b" x1="262.64" y1="15.46" x2="332.75" y2="85.57" xlink:href="#a" xlink:type="simple"/>
  </defs>
</svg>"##
        ),
    )?);

    Ok(())
}
