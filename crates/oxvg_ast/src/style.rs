//! Style and presentation attribute types.
use lightningcss::rules::CssRuleList;
use std::cell::RefCell;

use crate::element::Element;

#[cfg(feature = "selectors")]
use crate::{error::ComputedStylesError, get_attribute_mut, node};
#[cfg(feature = "selectors")]
use lightningcss::{
    declaration::DeclarationBlock,
    properties::{Property, PropertyId},
    rules::{self},
};
#[cfg(feature = "selectors")]
use oxvg_collections::{
    attribute::{Attr, AttrId, AttributeGroup, AttributeInfo},
    name::Prefix,
};
#[cfg(feature = "selectors")]
use std::collections::HashMap;

#[macro_export]
#[cfg(feature = "selectors")]
/// Returns the computed presentation attribute for a computed style in [`ComputedStyles`]
macro_rules! get_computed_style {
    ($computed_style:expr, $id:ident$(,)?) => {
        $computed_style
            .get(&oxvg_collections::attribute::AttrId::$id)
            .and_then(|(attr, mode)| match attr {
                oxvg_collections::attribute::Attr::$id(inner) => Some((inner, mode)),
                oxvg_collections::attribute::Attr::Unparsed { .. } => None,
                _ => unreachable!("{attr:?}"),
            })
    };
}

#[macro_export]
#[cfg(feature = "selectors")]
/// Returns whether the computed presentation attribute exists for a computed style
macro_rules! has_computed_style {
    ($computed_style:expr, $($id:ident)|+$(,)?) => {
        false $(|| $computed_style.get(&oxvg_collections::attribute::AttrId::$id).is_some())+
    }
}

#[macro_export]
/// Returns the computed presentation attribute for a computed style
macro_rules! get_computed_style_css {
    ($computed_style:expr, $id:ident$(($vp:ident))?$(,)?) => {
        $computed_style
            .get_by_property(&lightningcss::properties::PropertyId::$id$((lightningcss::vendor_prefix::VendorPrefix::$vp))?)
            .map(|(property, mode)| match property {
                lightningcss::properties::Property::$id(inner$(, lightningcss::vendor_prefix::VendorPrefix::$vp)?) => (inner, mode),
                _ => unreachable!(),
            })
    };
}

#[macro_export]
/// Returns whether the computed presentation attribute exists for a computed style
macro_rules! has_computed_style_css {
    ($computed_style:expr, $($id:ident$(($vp:ident))?)|+) => {
        false $(|| $computed_style
            .get_by_property(&lightningcss::properties::PropertyId::$id$((lightningcss::vendor_prefix::VendorPrefix::$vp))?).is_some())+
    }
}

#[cfg(feature = "selectors")]
#[derive(Default, Debug, PartialEq, Eq)]
/// A mode in which a style can be applied to an element
pub enum Mode {
    #[default]
    /// The application of a style based on an attribute, style attribute, or static stylesheet selector
    Static,
    /// The application of a style based on an at-rule or psuedo-class
    Dynamic,
}

#[cfg(feature = "selectors")]
#[derive(Debug, Clone)]
/// A style from either an attribute, style attribute, or static stylesheet selector
enum Static<'input> {
    /// A style from a style attribute or static stylesheet selector
    Css(Property<'input>),
    /// A style from an attribute
    Attr(Attr<'input>),
}

#[cfg(feature = "selectors")]
#[derive(Debug, Clone)]
/// A style that can be applied to an element, through either an attribute, style attribute, or stylesheet
enum Style<'i> {
    /// The style is declared directly through an attribute, style attribute, or static stylesheet selector
    Static(Static<'i>),
    /// The style is declared within a pseudo-class or at-rule
    Dynamic(Property<'i>),
}

#[cfg(feature = "selectors")]
#[derive(Default, Debug)]
/// A cloned collection of the different styles and how they're applied to a given element
pub struct ComputedStyles<'input> {
    /// Inherited styles (e.g. `p`'s color from `<div style="color: red;"><p /></div>`)
    inherited: HashMap<String, Style<'input>>,
    /// Styles (e.g. `<style>p { color: red }</style>`)
    declarations: HashMap<PropertyId<'input>, (u32, Style<'input>)>,
    /// Presentation attributes (e.g. `<p color="red" />`)
    attr: Vec<Attr<'input>>,
    /// Inline styles (e.g. `<p style="color: red;" />`)
    inline: Option<DeclarationBlock<'input>>,
    /// Important styles (e.g. `<style>p { color: red !important; }</style>`)
    important_declarations: HashMap<PropertyId<'input>, (u32, Style<'input>)>,
}

/// Gathers `<style>` declarations from the document
pub fn root<'input, 'arena>(
    root: &Element<'input, 'arena>,
) -> impl Iterator<Item = RefCell<CssRuleList<'input>>> + use<'input, 'arena> {
    root.breadth_first()
        .filter_map(|node| node.first_child())
        .filter_map(|node| node.style().cloned())
}

#[cfg(feature = "selectors")]
impl<'input> ComputedStyles<'input> {
    /// Include all sources of styles
    ///
    /// # Errors
    ///
    /// When styles contain bad selectors
    pub fn with_all(
        self,
        element: &Element<'input, '_>,
        styles: &[RefCell<CssRuleList<'input>>],
    ) -> Result<ComputedStyles<'input>, ComputedStylesError<'input>> {
        self.with_inline_style(element)
            .with_attribute(element)
            .with_style(element, styles)?
            .with_inherited(element, styles)
    }

    /// Include the computed styles of a parent element
    ///
    /// # Errors
    ///
    /// When styles contain bad selectors
    pub fn with_inherited(
        mut self,
        element: &Element<'input, '_>,
        styles: &[RefCell<CssRuleList<'input>>],
    ) -> Result<ComputedStyles<'input>, ComputedStylesError<'input>> {
        let Some(parent) = Element::parent_element(element) else {
            return Ok(self);
        };
        if parent.node_type() == node::Type::Document {
            return Ok(self);
        }
        let parent_styles = ComputedStyles::default().with_all(&parent, styles)?;
        self.inherited.extend(parent_styles.inherited);
        self.inherited.extend(
            parent_styles
                .declarations
                .into_iter()
                .map(|(id, value)| (id.name().to_string(), value.1)),
        );
        self.inherited
            .extend(parent_styles.attr.into_iter().map(|attr| {
                (
                    attr.name().to_string(),
                    Style::Static(Static::Attr(attr.clone())),
                )
            }));
        let (inline, important_inline) = parent_styles
            .inline
            .map(|style| (style.declarations, style.important_declarations))
            .unzip();
        self.inherited
            .extend(inline.into_iter().flatten().map(|property| {
                (
                    property.property_id().name().to_string(),
                    Style::Static(Static::Css(property)),
                )
            }));
        self.inherited.extend(
            parent_styles
                .important_declarations
                .into_iter()
                .map(|(id, value)| (id.name().to_string(), value.1)),
        );
        self.inherited
            .extend(important_inline.into_iter().flatten().map(|property| {
                (
                    property.property_id().name().to_string(),
                    Style::Static(Static::Css(property)),
                )
            }));
        Ok(self)
    }

    /// Include styles from the `style` attribute
    ///
    /// # Errors
    ///
    /// When styles contain bad selectors
    pub fn with_style(
        mut self,
        element: &Element<'input, '_>,
        styles: &[RefCell<CssRuleList<'input>>],
    ) -> Result<ComputedStyles<'input>, ComputedStylesError<'input>> {
        for css in styles {
            for s in &css.borrow().0 {
                self.with_nested_style(element, s, &mut Vec::new(), 0, &Mode::Static)?;
            }
        }
        Ok(self)
    }

    /// Include a style within a style scope
    fn with_nested_style(
        &mut self,
        element: &Element<'input, '_>,
        style: &rules::CssRule<'input>,
        selector: &mut Vec<String>,
        specificity: u32,
        #[allow(unused_variables)] mode: &Mode,
    ) -> Result<(), ComputedStylesError<'input>> {
        use crate::selectors::{SelectElement, Selector};
        use lightningcss::{printer::PrinterOptions, traits::ToCss};
        match style {
            rules::CssRule::Style(r) => {
                for s in &r.selectors.0 {
                    let this_selector =
                        s.to_css_string(PrinterOptions::default()).map_err(|e| {
                            ComputedStylesError::BadSelector {
                                reason: e.to_string(),
                                selector: r.selectors.clone(),
                            }
                        })?;
                    selector.push(this_selector);
                    let select = Selector::new(&selector.join("")).map_err(|e| {
                        ComputedStylesError::BadSelector {
                            reason: format!("{e:#?}"),
                            selector: r.selectors.clone(),
                        }
                    })?;
                    if !select.matches_naive(&SelectElement::new(element.clone())) {
                        continue;
                    }
                    self.add_declarations(&r.declarations, specificity + s.specificity(), mode);
                    selector.pop();
                }
                Ok(())
            }
            rules::CssRule::Container(rules::container::ContainerRule { rules, .. })
            | rules::CssRule::Media(rules::media::MediaRule { rules, .. }) => {
                for r in &rules.0 {
                    self.with_nested_style(element, r, selector, specificity, &Mode::Dynamic)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Include styles from a presentable attribute
    fn with_attribute(self, element: &Element<'input, '_>) -> ComputedStyles<'input> {
        let attr = element
            .attributes()
            .into_iter()
            .filter(|attr| {
                attr.name()
                    .attribute_group()
                    .contains(AttributeGroup::Presentation)
            })
            .map(|attr| attr.clone())
            .collect();
        ComputedStyles { attr, ..self }
    }

    fn with_inline_style(self, element: &Element<'input, '_>) -> ComputedStyles<'input> {
        let Some(style) = get_attribute_mut!(element, Style) else {
            return self;
        };
        ComputedStyles {
            inline: Some(style.0.clone()),
            ..self
        }
    }

    /// Gets only the style that may be applied from parent elements, ignoring any other
    /// sources of styles.
    pub fn get_inherited(&self, id: &str) -> Option<(Attr<'input>, Mode)> {
        if !AttributeGroup::Presentation
            .parse_attr_id(&Prefix::SVG, id.into())
            .info()
            .contains(AttributeInfo::Inheritable)
        {
            return None;
        }
        self.inherited
            .get(id)
            .cloned()
            .and_then(|style| match style {
                Style::Static(Static::Css(css)) => {
                    css.try_into().map(|attr| (attr, Mode::Static)).ok()
                }
                Style::Dynamic(css) => css.try_into().map(|attr| (attr, Mode::Dynamic)).ok(),
                Style::Static(Static::Attr(attr)) => {
                    if attr.name().info().contains(AttributeInfo::Inheritable) {
                        Some((attr, Mode::Static))
                    } else {
                        None
                    }
                }
            })
    }

    /// Gets the resolved style from a presentation attribute id.
    ///
    /// # Panics
    ///
    /// If conversions between property and attr fail
    pub fn get(&self, id: &AttrId) -> Option<(Attr<'input>, Mode)> {
        debug_assert!(id.attribute_group().contains(AttributeGroup::Presentation));
        let property_id = id.into();
        self.get_from_css_high_priori(&property_id)
            .map(|(css, mode)| {
                (
                    css.try_into()
                        .expect("attr convertable to property should also be able to convert back"),
                    mode,
                )
            })
            .or_else(|| {
                self.get_from_css_low_priori(&property_id)
                    .map(|(css, mode)| (css.try_into().unwrap(), mode))
            })
            .or_else(|| self.get_from_attr(id))
            .or_else(|| {
                // 7. Inherited
                self.get_inherited(id.local_name())
            })
    }

    /// Gets a computed style as a property if there's a matching [`PropertyId`]
    ///
    /// # Panics
    ///
    /// If conversions between property and attr fail
    pub fn get_by_property(&self, id: &PropertyId) -> Option<(Property<'input>, Mode)> {
        if let Ok(attr_id) = id.try_into() {
            return self.get(&attr_id).map(|(attr, mode)| {
                (
                    attr.try_into()
                        .expect("property convertable to attr should also be able to convert back"),
                    mode,
                )
            });
        }
        self.get_from_css_high_priori(id)
            .or_else(|| self.get_from_css_low_priori(id))
            .or_else(|| {
                self.inherited
                    .get(id.name())
                    .cloned()
                    .map(|style| match style {
                        Style::Static(Static::Css(css)) => (css, Mode::Static),
                        Style::Dynamic(css) => (css, Mode::Dynamic),
                        Style::Static(Static::Attr(_)) => {
                            unreachable!("should have called `ComputedStyles::get`")
                        }
                    })
            })
    }
    fn get_from_css_high_priori(&self, id: &PropertyId) -> Option<(Property<'input>, Mode)> {
        // 1. Inline important
        self.inline
            .as_ref()
            .and_then(|inline| {
                inline
                    .important_declarations
                    .iter()
                    .find(|css| css.property_id() == *id)
                    .cloned()
                    .map(|css| (css, Mode::Static))
            })
            .or_else(|| {
                // 2. Important declarations
                self.important_declarations
                    .get(id)
                    .cloned()
                    .map(|(_, style)| match style {
                        Style::Static(Static::Css(css)) => (css, Mode::Static),
                        Style::Dynamic(css) => (css, Mode::Dynamic),
                        Style::Static(Static::Attr(_)) => unreachable!(),
                    })
            })
            .or_else(|| {
                // 3. Inline
                self.inline.as_ref().and_then(|inline| {
                    inline
                        .declarations
                        .iter()
                        .find(|css| css.property_id() == *id)
                        .cloned()
                        .map(|css| (css, Mode::Static))
                })
            })
    }

    fn get_from_attr(&self, id: &AttrId) -> Option<(Attr<'input>, Mode)> {
        // 4. Attr
        self.attr
            .iter()
            .find(|attr| attr.name() == id)
            .cloned()
            .map(|attr| (attr, Mode::Static))
    }

    fn get_from_css_low_priori(&self, id: &PropertyId) -> Option<(Property<'input>, Mode)> {
        // 5. Declarations
        self.declarations
            .get(id)
            .cloned()
            .and_then(|(_, style)| match style {
                Style::Static(Static::Css(css)) => Some((css, Mode::Static)),
                Style::Dynamic(css) => Some((css, Mode::Dynamic)),
                Style::Static(Static::Attr(_)) => unreachable!(),
            })
    }

    fn add_declarations(
        &mut self,
        declarations: &lightningcss::declaration::DeclarationBlock<'input>,
        specificity: u32,
        mode: &Mode,
    ) {
        Self::set_declarations(
            &mut self.important_declarations,
            &declarations.important_declarations,
            specificity,
            mode,
        );
        Self::set_declarations(
            &mut self.declarations,
            &declarations.declarations,
            specificity,
            mode,
        );
    }

    fn set_declarations(
        record: &mut HashMap<PropertyId<'input>, (u32, Style<'input>)>,
        declarations: &[lightningcss::properties::Property<'input>],
        specificity: u32,
        mode: &Mode,
    ) {
        for d in declarations {
            let id = d.property_id();
            record.insert(id, (specificity, mode.style(Static::Css(d.clone()))));
        }
    }
}

#[cfg(feature = "selectors")]
impl Mode {
    /// # Panics
    /// If attempting to assign attribute to dynamic style
    fn style<'i>(&self, style: Static<'i>) -> Style<'i> {
        match self {
            Self::Static => Style::Static(style),
            Self::Dynamic => match style {
                Static::Attr(_) => panic!("cannot style attr as dynamic"),
                Static::Css(property) => Style::Dynamic(property),
            },
        }
    }

    /// Returns whether the source of a style is from an attribute or not
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static)
    }

    /// Returns whether the source of a style is from a stylesheet or not
    pub fn is_dynamic(&self) -> bool {
        !self.is_static()
    }
}
