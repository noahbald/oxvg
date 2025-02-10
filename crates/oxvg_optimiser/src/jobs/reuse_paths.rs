use std::collections::{BTreeMap, HashSet};

use derive_where::derive_where;
use lightningcss::{stylesheet::StyleSheet, visit_types};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    document::Document,
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use parcel_selectors::parser::Component;
use serde::{Deserialize, Serialize};

#[derive_where(Default, Clone)]
pub struct ReusePaths<E: Element> {
    enabled: bool,
    paths: BTreeMap<String, Vec<E>>,
    defs: Option<E>,
    hrefs: HashSet<String>,
}

impl<E: Element> Visitor<E> for ReusePaths<E> {
    type Error = String;

    fn prepare(&mut self, _document: &E, _context_flags: &mut ContextFlags) -> PrepareOutcome {
        if self.enabled {
            PrepareOutcome::use_style
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), Self::Error> {
        if element.prefix().is_some() {
            return Ok(());
        }

        if element.local_name().as_ref() == "path" {
            self.add_path(element);
        }

        if self.defs.is_none()
            && element.local_name().as_ref() == "defs"
            && Element::parent_element(element)
                .is_some_and(|e| e.prefix().is_none() && e.local_name().as_ref() == "svg")
        {
            self.defs = Some(element.clone());
        }

        if element.local_name().as_ref() == "use" {
            for attr in element.attributes().into_iter() {
                if attr
                    .prefix()
                    .as_ref()
                    .is_some_and(|p| p.as_ref() != "xlink")
                {
                    continue;
                }
                if attr.local_name().as_ref() != "href" {
                    continue;
                }
                let value = attr.value().as_ref();
                if value.len() > 1 && value.starts_with('#') {
                    self.hrefs.insert(value[1..].to_string());
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn exit_element(
        &mut self,
        element: &mut E,
        context: &mut Context<E>,
    ) -> Result<(), Self::Error> {
        if element.prefix().is_some()
            || !element.is_root()
            || element.local_name().as_ref() != "svg"
        {
            return Ok(());
        }
        if self.paths.is_empty() {
            return Ok(());
        }

        let document = context.root.as_document();
        let defs = if let Some(defs) = &self.defs {
            defs.clone()
        } else {
            let defs = document.create_element(E::Name::new(None, "defs".into()));
            element.insert(0, defs.clone().as_child());
            self.defs = Some(defs.clone());
            defs
        };

        let mut index = 0;
        let path_name: <E::Name as Name>::LocalName = "path".into();
        let id_name: <E::Name as Name>::LocalName = "id".into();
        let d_name = "d".into();
        let stroke_name = "stroke".into();
        let fill_name = "fill".into();
        for list in self.paths.values_mut() {
            if list.len() == 1 {
                continue;
            }

            let reusable_path = document.create_element(E::Name::new(None, path_name.clone()));
            defs.append(reusable_path.as_child());

            let mut is_id_protected = false;
            for attr in list[0].attributes().into_iter() {
                if attr.prefix().is_some() {
                    continue;
                }

                let value: &str = attr.value().as_ref();
                if attr.local_name().as_ref() == "id"
                    && !self.hrefs.contains(value)
                    && !HasId::has_id(&mut context.stylesheet, value)?
                {
                    is_id_protected = true;
                }
                if !matches!(attr.local_name().as_ref(), "fill" | "stroke" | "d" | "id") {
                    continue;
                }
                reusable_path.set_attribute(attr.name().clone(), attr.value().clone());
            }

            if is_id_protected {
                list[0].remove_attribute_local(&id_name);
            } else {
                reusable_path.set_attribute_local(id_name.clone(), format!("reuse-{index}").into());
                index += 1;
            }

            let new_id = reusable_path
                .get_attribute_local(&id_name)
                .expect("reusable path should be created with id");
            let new_id: E::Atom = format!("#{}", new_id.as_ref()).into();
            let href_name: <E::Name as Name>::LocalName = "href".into();
            let xlink_href_name = E::Name::new(Some("xlink".into()), "href".into());
            let use_name: <E::Name as Name>::LocalName = "use".into();
            for path in list {
                path.remove_attribute_local(&d_name);
                path.remove_attribute_local(&stroke_name);
                path.remove_attribute_local(&fill_name);

                if path.is_empty() && defs.contains(path) {
                    let attributes = path.attributes();
                    if attributes.is_empty() {
                        path.remove();
                    }
                    if attributes.len() == 1 {
                        let attr = attributes.into_iter().next().expect("checked length");
                        if attr.prefix().is_none() && attr.local_name().as_ref() == "id" {
                            path.remove();
                            let id = attr.value().as_ref();
                            for child in element
                                .select(&format!("[href='#{id}']"))
                                .map_err(|e| format!("{e:?}"))?
                            {
                                child.set_attribute_local(href_name.clone(), new_id.clone());
                            }
                            for child in element
                                .select(&format!("[xlink\\:href='#{id}']"))
                                .map_err(|e| format!("{e:?}"))?
                            {
                                child.set_attribute(xlink_href_name.clone(), new_id.clone());
                            }
                        }
                    }
                }

                path.set_attribute(xlink_href_name.clone(), new_id.clone());
                path.set_local_name(use_name.clone());
            }
        }

        if !defs.is_empty() {
            defs.set_attribute(
                E::Name::new(Some("xmlns".into()), "xlink".into()),
                "http://www.w3.org/1999/xlink".into(),
            );
        }

        Ok(())
    }
}

impl<E: Element> ReusePaths<E> {
    fn add_path(&mut self, element: &E) {
        let Some(d) = element.get_attribute_local(&"d".into()) else {
            return;
        };
        let d = d.as_ref();
        let fill = element.get_attribute_local(&"fill".into());
        let stroke = element.get_attribute_local(&"stroke".into());
        let mut key = String::with_capacity(
            d.len()
                + fill.as_ref().map(|a| a.as_ref().len()).unwrap_or_default()
                + stroke
                    .as_ref()
                    .map(|a| a.as_ref().len())
                    .unwrap_or_default()
                + 6,
        );
        key.push_str(d);
        key.push_str(";s:");
        if let Some(stroke) = stroke {
            key.push_str(stroke.as_ref());
        }
        key.push_str(";f:");
        if let Some(fill) = fill {
            key.push_str(fill.as_ref());
        }

        let list = self.paths.get_mut(&key);
        match list {
            Some(list) => list.push(element.clone()),
            None => {
                self.paths.insert(key, vec![element.clone()]);
            }
        }
    }
}

impl<'de, E: Element> Deserialize<'de> for ReusePaths<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let enabled = bool::deserialize(deserializer)?;
        Ok(Self {
            enabled,
            ..Self::default()
        })
    }
}

impl<E: Element> Serialize for ReusePaths<E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.enabled.serialize(serializer)
    }
}

struct HasId<'a> {
    found: bool,
    id: &'a str,
}

impl<'a> HasId<'a> {
    fn has_id(stylesheet: &mut Option<StyleSheet>, id: &'a str) -> Result<bool, String> {
        use lightningcss::visitor::Visitor;

        let Some(stylesheet) = stylesheet else {
            return Ok(false);
        };

        let mut has_id = Self { found: false, id };
        has_id.visit_stylesheet(stylesheet)?;
        Ok(has_id.found)
    }
}

impl<'i> lightningcss::visitor::Visitor<'i> for HasId<'_> {
    type Error = String;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(SELECTORS)
    }

    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'i>,
    ) -> Result<(), Self::Error> {
        if self.found {
            return Ok(());
        }

        self.found = selector.iter_raw_match_order().any(|c| match c {
            Component::ID(id) => id.as_ref() == self.id,
            _ => false,
        });
        Ok(())
    }
}

#[test]
fn reuse_paths() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "reusePaths": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path id="test0" d="M 10,50 l 20,30 L 20,30"/>
    <path transform="translate(10, 10)"
          d="M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70"/>
    <path transform="translate(20, 20)"
          d="M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70"/>
    <path d="M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70"/>
    <path id="test1" d="M 10,50 l 20,30 L 20,30"/>
    <path d="M 10,50 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70"/>
    <path d="M 20,30 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70"/>
    <g>
      <path id="test2" d="M 10,50 l 20,30 L 20,30"/>
    </g>
    <path d="M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "reusePaths": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path id="test0" d="M 10,50 l 20,30 L 20,30"/>
    <path id="test1" stroke="red" d="M 10,50 l 20,30 L 20,30"/>
    <path id="test2" stroke="blue" d="M 10,50 l 20,30 L 20,30"/>
    <path id="test3" d="M 10,50 l 20,30 L 20,30"/>
    <path id="test4" stroke="blue" d="M 10,50 l 20,30 L 20,30"/>
    <path id="test1" stroke="red" fill="green" d="M 10,50 l 20,30 L 20,30"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "reusePaths": true }"#,
        Some(
            r#"<svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
    <text>
        text element
    </text>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "reusePaths": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="-29.947 60.987 69.975 102.505">
  <defs></defs>
  <path fill="#000" d="M0 0v1h.5Z"/>
  <path fill="#000" d="M0 0v1h.5Z"/>
  <path fill="#000" d="M0 0v1h.5Z"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "reusePaths": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.0" viewBox="0 0 400 360">
  <defs>
    <path id="a" d="M51.94 428.2c14.5-32.39 36.88-59.5 64.38-81.96 13.76-11.23 65.04-24.09 73.86-16.58 9.45 8.06 13.45 26.18 5.53 38.45-1.23 1.9-37.38 26.83-39.1 28.32-2.19 1.9-38.65 17.58-43.76 19.51-14.02 5.28-29.47 10.43-44.31 12.71-3.19.5-14.98 3.85-16.6-.45z"/>
    <path id="b" d="M51.94 428.2c14.5-32.39 36.88-59.5 64.38-81.96 13.76-11.23 65.04-24.09 73.86-16.58 9.45 8.06 13.45 26.18 5.53 38.45-1.23 1.9-37.38 26.83-39.1 28.32-2.19 1.9-38.65 17.58-43.76 19.51-14.02 5.28-29.47 10.43-44.31 12.71-3.19.5-14.98 3.85-16.6-.45z"/>
    <clipPath id="c">
      <use xlink:href="#b" width="100%" height="100%" overflow="visible"/>
    </clipPath>
  </defs>
  <g transform="matrix(.491 0 0 .491 10.63 63.15)">
    <use xlink:href="#b" width="100%" height="100%" fill="#fff" fill-rule="evenodd" clip-rule="evenodd" overflow="visible"/>
    <path fill="none" stroke="#c8cacc" stroke-miterlimit="3.86" stroke-width="66.34" d="M48.33 412.36c14.5-32.39 36.89-59.5 64.39-81.96 13.75-11.23 65.03-24.09 73.85-16.58 9.45 8.06 13.45 26.18 5.53 38.45-1.22 1.9-37.38 26.83-39.09 28.32-2.2 1.9-38.65 17.58-43.77 19.51-14.01 5.28-29.47 10.44-44.3 12.71-3.2.5-14.99 3.85-16.61-.45z" clip-path="url(#c)"/>
  </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "reusePaths": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="-29.947 60.987 69.975 102.505">
  <g transform="translate(-59 64)">
    <g id="b">
      <path id="a" fill="#000" d="M0 0v1h.5Z" transform="rotate(18 3.157 -.5)"/>
      <use xlink:href="#a" width="1" height="1" transform="scale(-1 1)"/>
    </g>
    <use xlink:href="#b" width="1" height="1" transform="rotate(72)"/>
    <use xlink:href="#b" width="1" height="1" transform="rotate(-72)"/>
    <use xlink:href="#b" width="1" height="1" transform="rotate(144)"/>
    <use xlink:href="#b" width="1" height="1" transform="rotate(-144)"/>
  </g>
  <path id="c" fill="#000" d="M0 0v1h.5Z" transform="rotate(18 3.157 -.5)"/>
  <use xlink:href="#c" width="1" height="1" transform="scale(-1 1)"/>
</svg>"##
        ),
    )?);

    Ok(())
}
