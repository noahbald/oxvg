use std::marker::PhantomData;

use oxvg_ast::{
    attribute::{Attr, Attributes},
    document::Document,
    element::Element,
    name::Name,
    node::Node,
    visitor::{Context, Visitor},
};
use oxvg_collections::allowed_content::ELEMS;
use phf::{phf_map, phf_set};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct RemoveXlink {
    #[serde(default = "bool::default")]
    include_legacy: bool,
}

struct State<'o, 'arena, E: Element<'arena>> {
    options: &'o RemoveXlink,
    xlink_prefixes: Vec<<E::Name as Name>::Prefix>,
    overridden_prefixes: Vec<<E::Name as Name>::Prefix>,
    used_in_legacy_element: Vec<<E::Name as Name>::Prefix>,
    marker: PhantomData<&'arena ()>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for RemoveXlink {
    type Error = String;

    fn document(
        &mut self,
        document: &mut E,
        context: &Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        State {
            options: &*self,
            xlink_prefixes: vec![],
            overridden_prefixes: vec![],
            used_in_legacy_element: vec![],
            marker: PhantomData,
        }
        .start(document, context.info)
        .map(|_| ())
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'_, 'arena, E> {
    type Error = String;

    fn element(
        &mut self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        for attr in element.attributes().into_iter() {
            if let Some(prefix) = attr.prefix() {
                if prefix.as_ref() != "xmlns" {
                    continue;
                }

                let prefix_name = attr.local_name().as_ref().into();
                if attr.value().as_ref() == XLINK_NAMESPACE {
                    self.xlink_prefixes.push(prefix_name);
                } else if self.xlink_prefixes.contains(&prefix_name) {
                    self.overridden_prefixes.push(prefix_name);
                }
            }
        }

        if self
            .overridden_prefixes
            .iter()
            .any(|p| self.xlink_prefixes.contains(p))
        {
            return Ok(());
        }

        self.handle_show(element);
        self.handle_title(element, context);
        self.handle_href(element);

        Ok(())
    }

    fn exit_element(
        &mut self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        element.attributes().retain(|attr| {
            let Some(prefix) = attr.prefix() else {
                return true;
            };

            if !self.options.include_legacy
                && self.xlink_prefixes.contains(prefix)
                && !self.overridden_prefixes.contains(prefix)
                && !self.used_in_legacy_element.contains(prefix)
            {
                return false;
            }

            let value = attr.value();
            if prefix.as_ref() == "xmlns"
                && !self.used_in_legacy_element.contains(&value.as_ref().into())
            {
                if value.as_ref() == XLINK_NAMESPACE {
                    self.xlink_prefixes.retain(|p| p.as_ref() != value.as_ref());
                    return false;
                }

                self.overridden_prefixes.retain(|p| p != prefix);
            }

            true
        });

        Ok(())
    }
}

impl<'arena, E: Element<'arena>> State<'_, 'arena, E> {
    fn handle_show(&self, element: &E) {
        let element_name = element.qual_name().formatter().to_string();
        let target_name = "target".into();
        let mut show_handled = element.has_attribute_local(&target_name);
        let mut new_target = None;
        element.attributes().retain(|attr| {
            let Some(prefix) = attr.prefix() else {
                return true;
            };
            if attr.local_name().as_ref() != "show" || !self.xlink_prefixes.contains(prefix) {
                return true;
            }
            if show_handled {
                return false;
            }

            let mapping = SHOW_TO_TARGET.get(attr.value());
            let default_mapping = ELEMS
                .get(&element_name)
                .and_then(|m| m.defaults.as_ref())
                .and_then(|m| m.get("target"));
            if mapping.is_some() {
                show_handled = true;
                if mapping != default_mapping {
                    new_target = mapping;
                }
            }

            false
        });
        if let Some(new_target) = new_target {
            element.set_attribute_local(target_name, (*new_target).into());
        }
    }

    fn handle_title(&self, element: &E, context: &Context<'arena, '_, '_, E>) {
        element.attributes().retain(|attr| {
            let Some(prefix) = attr.prefix() else {
                return true;
            };
            if attr.local_name().as_ref() != "title" || !self.xlink_prefixes.contains(prefix) {
                return true;
            }

            let has_title = element.child_nodes_iter().all(|child| {
                child
                    .element()
                    .is_some_and(|e| e.prefix().is_none() && e.local_name().as_ref() == "title")
            });
            if has_title {
                return false;
            }

            let title = context
                .root
                .as_document()
                .create_element(E::Name::new(None, "title".into()), &context.info.arena);
            title.set_text_content(attr.value().clone(), &context.info.arena);
            element.clone().insert(0, title.as_child());
            false
        });
    }

    fn handle_href(&mut self, element: &E) {
        let exclude_legacy = !self.options.include_legacy
            && element.prefix().is_none()
            && LEGACY_ELEMENTS.contains(element.local_name());
        let mut has_href = false;
        let mut new_href = None;
        element.attributes().retain(|attr| {
            let Some(prefix) = attr.prefix() else {
                has_href = has_href || attr.value().as_ref() == "href";
                return true;
            };
            if attr.local_name().as_ref() != "href" || !self.xlink_prefixes.contains(prefix) {
                log::debug!("retaining {:?}, not a recorded prefix", attr);
                return true;
            }
            if exclude_legacy {
                self.used_in_legacy_element.push(prefix.clone());
                return true;
            }

            new_href = Some(attr.value().clone());

            false
        });

        if !has_href {
            if let Some(new_href) = new_href {
                element.set_attribute_local("href".into(), new_href);
            }
        }
    }
}

static XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";
static SHOW_TO_TARGET: phf::Map<&'static str, &'static str> = phf_map! {
    "new" => "_blank",
    "replace" => "_self",
};
static LEGACY_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "cursor",
    "filter",
    "font-face-uri",
    "glyphRef",
    "tref",
};

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
