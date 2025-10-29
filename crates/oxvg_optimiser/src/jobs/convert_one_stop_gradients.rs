use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use lightningcss::{
    properties::{custom::TokenOrValue, Property},
    visit_types,
    visitor::Visit,
};
use oxvg_ast::{
    atom::Atom,
    attribute::{
        content_type::ContentType,
        data::{
            core::{Color, Paint},
            inheritable::Inheritable,
            Attr, AttrId,
        },
    },
    element::{data::ElementId, Element},
    get_attribute, get_attribute_mut,
    name::{Prefix, QualName},
    node::AllocationID,
    style::{ComputedStyles, Mode},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(transparent)]
/// Converts `linearGradient` and `radialGradient` nodes that are a solid colour
/// to the equivalent colour.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct ConvertOneStopGradients(pub bool);

#[derive(Default)]
struct State<'input, 'arena> {
    /// Parent defs with removed gradients
    effected_defs: RefCell<HashMap<AllocationID, Element<'input, 'arena>>>,
    /// All defs and their associated parent
    all_defs: RefCell<HashMap<AllocationID, Element<'input, 'arena>>>,
    gradients_to_detach: RefCell<HashMap<AllocationID, Element<'input, 'arena>>>,
    xlink_href_count: Cell<usize>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for ConvertOneStopGradients {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        info: &Info<'input, 'arena>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State::default().start(&mut document.clone(), info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'input, 'arena> {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _info: &Info<'input, 'arena>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(PrepareOutcome::use_style)
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if element.has_attribute(&AttrId::XLinkHref) {
            self.xlink_href_count.set(self.xlink_href_count.get() + 1);
        }
        let name = element.qual_name();
        if *name == ElementId::Defs {
            self.all_defs
                .borrow_mut()
                .insert(element.id(), element.clone());
            return Ok(());
        }
        if *name != ElementId::LinearGradient && *name != ElementId::RadialGradient {
            return Ok(());
        }

        let mut stops = element
            .child_elements_iter()
            .filter(|child| *child.qual_name() == ElementId::Stop);

        let href = get_attribute!(element, XLinkHref).or_else(|| get_attribute!(element, Href));
        let effective_node = if stops.next().is_none() {
            if let Some(href) = href {
                if href.starts_with('#') {
                    context.root.breadth_first().find(|element| {
                        get_attribute!(element, Id).is_some_and(|id| id.as_str() == &href[1..])
                    })
                } else {
                    Some(element.clone())
                }
            } else {
                Some(element.clone())
            }
        } else {
            Some(element.clone())
        };

        let mut gradients_to_detach = self.gradients_to_detach.borrow_mut();
        let Some(effective_node) = effective_node else {
            log::debug!("no effective nodes for gradient");
            gradients_to_detach.insert(element.id(), element.clone());
            return Ok(());
        };

        let effective_stops: Vec<_> = effective_node
            .child_elements_iter()
            .filter(|child| *child.qual_name() == ElementId::Stop)
            .collect();

        if effective_stops.len() != 1 {
            log::debug!("skipping, multiple/zero stops for gradient");
            return Ok(());
        }

        if let Some(parent) = element.parent_element() {
            if *parent.qual_name() == ElementId::Defs {
                self.effected_defs.borrow_mut().insert(parent.id(), parent);
            }
        }

        gradients_to_detach.insert(element.id(), element.clone());

        let color = get_color(context, &effective_stops)?;
        let Some(id) = get_attribute!(element, Id) else {
            log::debug!("skipping reference updates, no id");
            return Ok(());
        };
        log::debug!("updating colors: {color:?}");
        let Some(color) = color else { return Ok(()) };
        update_color_references(context, &color, &id);
        update_style_references(context, &color, &id)
    }

    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if *element.qual_name() != ElementId::Svg {
            return Ok(());
        }

        for gradient in self.gradients_to_detach.borrow().values() {
            if gradient.has_attribute(&AttrId::XLinkHref) {
                self.xlink_href_count.set(self.xlink_href_count.get() - 1);
            }

            gradient.remove();
        }

        if self.xlink_href_count.get() == 0 {
            element.remove_attribute(&AttrId::Unknown(QualName {
                prefix: Prefix::XMLNS,
                local: Atom::Static("xlink"),
            }));
        }

        let effected_defs = self.effected_defs.borrow();
        for def in self.all_defs.borrow().values() {
            if !def.has_child_elements() && effected_defs.contains_key(&def.id()) {
                def.remove();
            }
        }

        Ok(())
    }
}

fn get_color<'a, 'input, 'arena>(
    context: &'a mut Context<'input, 'arena, '_>,
    effective_stops: &[Element<'input, 'arena>],
) -> Result<Option<Color>, JobsError<'input>> {
    let effective_stop = effective_stops.first().expect("len should be 1");
    let computed_styles = ComputedStyles::default()
        .with_all(effective_stop, &context.stylesheet)
        .map_err(JobsError::ComputedStylesError)?;

    if let Some((stop_color, Mode::Static)) = computed_styles.get(&AttrId::StopColor) {
        return Ok(match stop_color {
            Attr::StopColor(Inheritable::Defined(color)) => Some(color),
            Attr::CSSUnknown { value, .. } => value
                .0
                 .0
                .into_iter()
                .filter_map(|token| match token {
                    TokenOrValue::Color(color) => Some(color),
                    _ => None,
                })
                .next(),
            _ => None,
        });
    }
    Ok(None)
}

fn update_color_references<'input, 'arena>(
    context: &mut Context<'input, 'arena, '_>,
    color: &Color,
    url: &str,
) {
    for element in context.root.breadth_first() {
        for mut attr in element.attributes().into_iter_mut() {
            let value = attr.value_mut();
            let ContentType::Paint(mut attr_color) = value else {
                continue;
            };
            if let Paint::Url { url: attr_url, .. } = &mut *attr_color {
                if attr_url.url == url {
                    *attr_color = Paint::Color(color.clone());
                }
            }
        }
    }
}

struct VisitPaint<'a> {
    url: &'a str,
    color: &'a Color,
}
impl<'i> lightningcss::visitor::Visitor<'i> for VisitPaint<'_> {
    type Error = JobsError<'i>;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(PROPERTIES)
    }

    fn visit_property(&mut self, property: &mut Property<'i>) -> Result<(), Self::Error> {
        let (Property::Fill(paint) | Property::Stroke(paint)) = property else {
            return Ok(());
        };
        if let Paint::Url { url: attr_url, .. } = paint {
            if attr_url.url == self.url {
                *paint = Paint::Color(self.color.clone());
            }
        }
        Ok(())
    }
}
fn update_style_references<'input, 'arena>(
    context: &Context<'input, 'arena, '_>,
    color: &Color,
    url: &str,
) -> Result<(), JobsError<'input>> {
    for element in context.root.breadth_first() {
        let mut style = get_attribute_mut!(element, Style);
        let Some(oxvg_ast::attribute::data::core::Style(style)) = style.as_deref_mut() else {
            continue;
        };
        style.visit(&mut VisitPaint { url, color })?;
    }
    Ok(())
}

#[test]
fn convert_one_stop_gradients() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertOneStopGradients": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" version="1.1" width="744.09448" height="1052.3622">
  <!-- Convert both a one-stop gradient configured from attribute and styles. -->
  <defs>
    <linearGradient id="a">
      <stop stop-color="#ddc4cc"/>
    </linearGradient>
    <linearGradient id="b">
      <stop style="stop-color:#a8c4cc"/>
    </linearGradient>
  </defs>
  <rect width="150" height="150" x="350" y="350" fill="url(#a)"/>
  <rect width="150" height="150" x="50" y="350" style="fill:url(#b)"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertOneStopGradients": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1" width="744.09448" height="1052.3622">
  <!-- Convert a one-stop gradient that references another one-stop gradient. -->
  <!-- Remove xlink:href namespace since we remove the only reference to it. -->
  <defs>
    <linearGradient id="a">
      <stop style="stop-color:#a8c4cc"/>
    </linearGradient>
    <linearGradient x1="353.83112" y1="396.85037" x2="496.56262" y2="396.85037" id="b" xlink:href="#a"/>
  </defs>
  <rect width="150" height="150" x="350" y="350" style="fill:url(#b)"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertOneStopGradients": true }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg"
  xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1" width="744.09448" height="1052.3622">
  <!-- If a one-stop gradient has the color defined via both attribute and style, style takes precedence. -->
  <defs>
    <linearGradient id="a">
      <stop stop-color="#ff0000" style="stop-color:#00ff00"/>
    </linearGradient>
    <linearGradient x1="353.83112" y1="396.85037" x2="496.56262" y2="396.85037" id="b" xlink:href="#a"/>
  </defs>
  <rect width="150" height="150" x="350" y="350" style="fill:url(#b)"/>
</svg>"##
        ),
    )?);

    Ok(())
}
