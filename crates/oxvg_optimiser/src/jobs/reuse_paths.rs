use std::{cell::RefCell, collections::HashSet};

use lightningcss::{properties::svg::SVGPaint, rules::CssRuleList, visit_types};
use oxvg_ast::{
    element::Element,
    get_attribute, get_attribute_mut, is_element, remove_attribute, set_attribute,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core::{NonWhitespace, Url},
        inheritable::Inheritable,
        path, Attr, AttrId,
    },
    element::ElementId,
    name::{Prefix, QualName, NS},
};
use oxvg_path::Path;
use parcel_selectors::parser::Component;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Default, Debug)]
struct State<'input, 'arena> {
    paths: RefCell<Vec<(Key<'input>, Vec<Element<'input, 'arena>>)>>,
    defs: RefCell<Option<Element<'input, 'arena>>>,
    hrefs: RefCell<HashSet<Url<'input>>>,
}

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "serde", serde(transparent))]
/// For duplicate `<path>` elements, replaces it with a `<use>` that references a single
/// `<path>` definition.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// If a path has an invalid id.
pub struct ReusePaths(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for ReusePaths {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State::default().start_with_context(document, context)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'input, 'arena> {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        Ok(PrepareOutcome::none)
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        match element.qual_name().unaliased() {
            ElementId::Path => self.add_path(element),
            ElementId::Defs => {
                if self.defs.borrow().is_none()
                    && element
                        .parent_element()
                        .is_some_and(|parent| is_element!(parent, Svg))
                {
                    self.defs.replace(Some(element.clone()));
                }
            }
            ElementId::Use => {
                let mut hrefs = self.hrefs.borrow_mut();
                for attr in element.attributes() {
                    let (Attr::Href(value) | Attr::XLinkHref(value)) = attr.unaliased() else {
                        continue;
                    };
                    if value.len() > 1 && value.starts_with('#') {
                        hrefs.insert(value.clone());
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !element.is_root() && !is_element!(element, Svg) {
            return Ok(());
        }
        let mut paths = self.paths.borrow_mut();
        if paths.is_empty() {
            return Ok(());
        }

        let document = context.root.as_document();
        let defs = self.defs.borrow();
        let defs = if let Some(defs) = &*defs {
            defs.clone()
        } else {
            drop(defs);
            let defs = document.create_element(ElementId::Defs, &context.info.allocator);
            element.insert(0, &defs);
            self.defs.replace(Some(defs.clone()));
            defs
        };

        let mut index = 0;
        let hrefs = self.hrefs.borrow();
        for (_, list) in paths.iter_mut() {
            if list.len() == 1 {
                continue;
            }

            let reusable_path = document.create_element(ElementId::Path, &context.info.allocator);
            defs.append(reusable_path.0);

            let mut is_id_protected = false;
            for attr in list[0].attributes() {
                match attr.unaliased() {
                    Attr::Id(id) => {
                        if !hrefs.contains(&id.0)
                            && !HasId::has_id(&context.query_has_stylesheet_result, id)?
                        {
                            is_id_protected = true;
                        }
                    }
                    Attr::Fill(_) | Attr::Stroke(_) | Attr::D(_) => {}
                    _ => continue,
                }
                reusable_path.set_attribute(attr.clone());
            }

            if is_id_protected {
                remove_attribute!(list[0], Id);
            } else {
                set_attribute!(
                    reusable_path,
                    Id(NonWhitespace(format!("reuse-{index}").into()))
                );
                index += 1;
            }

            let new_id_attr =
                get_attribute!(reusable_path, Id).expect("reusable path should be created with id");
            let new_id: Atom<'input> = format!("#{}", new_id_attr.0).into();
            drop(new_id_attr);
            for path in list {
                remove_attribute!(path, D);
                remove_attribute!(path, Stroke);
                remove_attribute!(path, Fill);

                if path.is_empty() && defs.contains(path) {
                    let attributes = path.attributes();
                    if attributes.is_empty() {
                        log::debug!("removing empty path");
                        path.remove();
                    }
                    if attributes.len() == 1 {
                        let attr = attributes.into_iter().next().expect("checked length");
                        if let Attr::Id(NonWhitespace(id)) = attr.unaliased() {
                            log::debug!("removing referenced path");
                            let old_url = format!("#{id}");
                            drop(attr);
                            path.remove();
                            for child in element.breadth_first() {
                                let mut href = get_attribute_mut!(child, Href)
                                    .or_else(|| get_attribute_mut!(child, XLinkHref));
                                if let Some(url) = href.as_deref_mut() {
                                    if url.as_str() == old_url {
                                        *url = new_id.clone();
                                    }
                                }
                            }
                        }
                    }
                }

                set_attribute!(path, XLinkHref(new_id.clone()));
                *path = path.set_local_name(ElementId::Use, &context.info.allocator);
            }
        }

        if !defs.is_empty() {
            defs.set_attribute(Attr::Unparsed {
                attr_id: AttrId::Unknown(QualName {
                    prefix: Prefix::XMLNS,
                    local: Prefix::XLink.value().unwrap(),
                }),
                value: NS::XLink.uri().clone(),
            });
        }

        Ok(())
    }
}

impl<'input, 'arena> State<'input, 'arena> {
    fn add_path(&self, element: &Element<'input, 'arena>) {
        let Some(path::Path(path)) = get_attribute!(element, D).as_deref().cloned() else {
            return;
        };
        let fill = get_attribute!(element, Fill).as_deref().cloned();
        let stroke = get_attribute!(element, Stroke).as_deref().cloned();
        let key = Key { path, stroke, fill };
        let mut paths = self.paths.borrow_mut();
        let list = paths.iter_mut().find(|(k, _)| k == &key);
        match list {
            Some((_, list)) => list.push(element.clone()),
            None => paths.push((key, vec![element.clone()])),
        }
    }
}

#[derive(PartialEq, Debug)]
struct Key<'input> {
    path: Path,
    stroke: Option<Inheritable<SVGPaint<'input>>>,
    fill: Option<Inheritable<SVGPaint<'input>>>,
}

struct HasId<'a> {
    found: bool,
    id: &'a str,
}

impl<'a> HasId<'a> {
    fn has_id<'input>(
        stylesheet: &[RefCell<CssRuleList<'input>>],
        id: &'a str,
    ) -> Result<bool, JobsError<'input>> {
        use lightningcss::visitor::Visit as _;
        let mut has_id = Self { found: false, id };
        for css_rule_list in stylesheet {
            css_rule_list.borrow_mut().visit(&mut has_id)?;
            if has_id.found {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl<'input> lightningcss::visitor::Visitor<'input> for HasId<'_> {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(SELECTORS)
    }

    fn visit_selector(
        &mut self,
        selector: &mut lightningcss::selector::Selector<'input>,
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
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-29.947 60.987 69.975 102.505">
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
