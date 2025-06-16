use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    marker::PhantomData,
};

use derive_where::derive_where;
use itertools::Itertools as _;
use lightningcss::{
    printer::PrinterOptions,
    properties::{
        custom::{CustomProperty, CustomPropertyName, TokenOrValue},
        Property, PropertyId,
    },
    traits::ToCss as _,
};
use oxvg_ast::{
    attribute::Attr,
    element::Element,
    name::Name as _,
    style::{ComputedStyles, Id, PresentationAttr, PresentationAttrId, Static, Style},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::{AttrsGroups, COLORS_PROPS};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

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

#[derive_where(Default)]
struct State<'arena, E: Element<'arena>> {
    /// Parent defs with removed gradients
    effected_defs: RefCell<HashSet<E>>,
    /// All defs and their associated parent
    all_defs: RefCell<HashSet<E>>,
    gradients_to_detach: RefCell<HashSet<E>>,
    xlink_href_count: Cell<usize>,
    marker: PhantomData<&'arena ()>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for ConvertOneStopGradients {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        if self.0 {
            State::default().start(&mut document.clone(), info, None)?;
        }
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'arena, E> {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(PrepareOutcome::use_style)
    }

    fn element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        let xlink_attr = <E::Attr as Attr>::Name::new(Some("xlink".into()), "href".into());
        if element.has_attribute(&xlink_attr) {
            self.xlink_href_count.set(self.xlink_href_count.get() + 1);
        }
        if element.local_name().as_ref() == "defs" {
            self.all_defs.borrow_mut().insert(element.clone());
            return Ok(());
        }
        if element.local_name().as_ref() != "linearGradient"
            && element.local_name().as_ref() != "radialGradient"
        {
            return Ok(());
        }

        let mut stops = element
            .child_elements_iter()
            .filter(|child| child.prefix().is_none() && child.local_name().as_ref() == "stop");

        let href = element
            .get_attribute_node(&xlink_attr)
            .or_else(|| element.get_attribute_node_local(&"href".into()));
        let effective_node = if stops.next().is_none() {
            if let Some(href) = href {
                if href.value().starts_with('#') {
                    context
                        .root
                        .select(href.value())
                        .ok()
                        .and_then(|mut i| i.next())
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
            gradients_to_detach.insert(element.clone());
            return Ok(());
        };

        let effective_stops: Vec<_> = effective_node
            .child_elements_iter()
            .filter(|child| child.prefix().is_none() && child.local_name().as_ref() == "stop")
            .collect();

        if effective_stops.len() != 1 {
            log::debug!("skipping, multiple/zero stops for gradient");
            return Ok(());
        }

        if let Some(parent) = element.parent_element() {
            if parent.prefix().is_none() && parent.local_name().as_ref() == "defs" {
                self.effected_defs.borrow_mut().insert(parent);
            }
        }

        gradients_to_detach.insert(element.clone());

        let color = get_color(context, &effective_stops);
        let Some(id) = element.get_attribute_local(&"id".into()) else {
            log::debug!("skipping reference updates, no id");
            return Ok(());
        };
        log::debug!("updating colors: {color:?}");
        let selector_val = format!("url(#{id})");
        update_color_references(context, color.as_ref(), &selector_val)?;
        update_style_references(context, color.as_ref(), &selector_val)
    }

    fn exit_element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if element.prefix().is_some() || element.local_name().as_ref() != "svg" {
            return Ok(());
        }

        let xlink_href = <E::Attr as Attr>::Name::new(Some("xlink".into()), "href".into());
        for gradient in self.gradients_to_detach.borrow().iter() {
            if gradient.has_attribute(&xlink_href) {
                self.xlink_href_count.set(self.xlink_href_count.get() - 1);
            }

            gradient.remove();
        }

        if self.xlink_href_count.get() == 0 {
            element.remove_attribute(&<E::Attr as Attr>::Name::new(
                Some("xmlns".into()),
                "xlink".into(),
            ));
        }

        let effected_defs = self.effected_defs.borrow();
        for def in self.all_defs.borrow().iter() {
            if !def.has_child_elements() && effected_defs.contains(def) {
                def.remove();
            }
        }

        Ok(())
    }
}

fn get_color<'arena, E: Element<'arena>>(
    context: &mut Context<'arena, '_, '_, E>,
    effective_stops: &[E],
) -> Option<Result<E::Atom, String>> {
    let effective_stop = effective_stops.first().expect("len should be 1");
    let computed_styles = ComputedStyles::default().with_all(
        effective_stop,
        &context.stylesheet,
        context.element_styles,
    );

    let stop_color_unknown = PropertyId::Custom(CustomPropertyName::Unknown("stop-color".into()));
    // FIXME: Use `get_computed_styles_factory!`
    computed_styles
        .important_declarations
        .get(&stop_color_unknown)
        .map(|p| &p.1)
        .or_else(|| computed_styles.inline_important.get(&stop_color_unknown))
        .or_else(|| computed_styles.inline.get(&stop_color_unknown))
        .or_else(|| {
            computed_styles
                .declarations
                .get(&stop_color_unknown)
                .map(|p| &p.1)
        })
        .or_else(|| computed_styles.attr.get(&PresentationAttrId::StopColor))
        .or_else(|| computed_styles.inherited.get(&Id::CSS(stop_color_unknown)))
        .or_else(|| {
            computed_styles
                .inherited
                .get(&Id::Attr(PresentationAttrId::StopColor))
        })
        .and_then(|style| match style {
            Style::Static(style) => match style {
                Static::Css(Property::Custom(CustomProperty { value, .. })) => {
                    value.0.first().and_then(|token| match token {
                        TokenOrValue::Color(color) => Some(color),
                        _ => None,
                    })
                }
                Static::Attr(PresentationAttr::StopColor(color)) => Some(color),
                _ => None,
            },
            Style::Dynamic(_) => None,
        })
        .map(|color| {
            color
                .to_css_string(PrinterOptions::default())
                .map_err(|e| e.to_string())
                .map(Into::into)
        })
}

fn update_color_references<'arena, E: Element<'arena>>(
    context: &mut Context<'arena, '_, '_, E>,
    color: Option<&Result<E::Atom, String>>,
    selector_val: &str,
) -> Result<(), String> {
    let selector = COLORS_PROPS
        .iter()
        .map(|attr| format!(r#"[{attr}="{selector_val}"]"#))
        .join(",");
    let elements = match context.root.select(&selector) {
        Ok(elements) => elements,
        Err(err) => {
            log::debug!("unable to parse selector `{selector}`: {err:?}");
            return Ok(());
        }
    };
    for element in elements {
        for attr in &COLORS_PROPS {
            let attr_name = (*attr).into();
            let Some(mut attr) = element.get_attribute_node_local_mut(&attr_name) else {
                continue;
            };
            if attr.value().as_ref() != selector_val {
                continue;
            }

            if let Some(color) = color {
                attr.set_value(color.as_ref().map_err(ToString::to_string)?.clone());
            } else {
                drop(attr);
                element.remove_attribute_local(&attr_name);
            }
        }
    }
    Ok(())
}

fn update_style_references<'arena, E: Element<'arena>>(
    context: &Context<'arena, '_, '_, E>,
    color: Option<&Result<E::Atom, String>>,
    selector_val: &str,
) -> Result<(), String> {
    let styled_selector = format!(r#"[style*="{selector_val}"]"#);
    let styled_elements = match context.root.select(&styled_selector) {
        Ok(elements) => elements,
        Err(err) => {
            log::debug!("unable to parse selector `{styled_selector}`: {err:?}");
            return Ok(());
        }
    };
    let style = "style".into();
    for element in styled_elements {
        let Some(mut attr) = element.get_attribute_node_local_mut(&style) else {
            continue;
        };

        let color = match color {
            Some(Ok(ref color)) => color,
            Some(Err(err)) => return Err(err.to_string()),
            None => *AttrsGroups::Presentation
                .defaults()
                .unwrap()
                .get("stop-color")
                .unwrap(),
        };
        let color = attr.value().as_ref().replace(selector_val, color).into();
        attr.set_value(color);
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
