//! Style and presentation attribute types.
use cfg_if::cfg_if;
use lightningcss::{
    declaration, printer,
    properties::{self, custom::CustomPropertyName, Property, PropertyId},
    rules::{self, CssRule, CssRuleList},
    traits::ToCss,
    values::{length::LengthPercentage, shape::FillRule},
    vendor_prefix::VendorPrefix,
};
use std::collections::HashMap;

use crate::{
    attribute::{
        data::{
            core::{Angle, Color, Length, Number, Opacity, Paint},
            inheritable::Inheritable,
            list_of::{Comma, ListOf},
            presentation::{
                AlignmentBaseline, BaselineShift, Clip, ClipPath, ColorInterpolation, ColorProfile,
                Cursor, Direction, Display, DominantBaseline, EnableBackground, FilterList, Font,
                FontFamily, FontSize, FontStretch, FontStyle, FontVariant, FontWeight, Marker,
                Mask, Overflow, PaintOrder, PointerEvents, Position, Rendering, ShapeRendering,
                Spacing, StrokeDasharray, StrokeLinecap, StrokeLinejoin, TextAnchor,
                TextDecoration, UnicodeBidi, VectorEffect, Visibility, WritingMode,
            },
            Attr, AttrId,
        },
        AttributeGroup,
    },
    element::Element,
};

macro_rules! define_presentation_attrs {
    (
        $(
            $name:literal: $attr:ident($type:ty $(, $vp:ty)?) $((inheritable: $inherit:ident))? $(/ $is_matching_property_id:ident)?,
        )+
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        /// A presentation attribute ID.
        pub enum PresentationAttrId {
            $(
                #[doc=concat!("The `", $name, "` attribute.")]
                $attr,
            )+
        }

        impl PresentationAttrId {
            fn attr_id(&self) -> AttrId<'static> {
                match self {
                    $(Self::$attr => AttrId::$attr,)+
                }
            }
        }

        impl<'i> TryFrom<&PropertyId<'i>> for PresentationAttrId {
            type Error = ();

            fn try_from(value: &PropertyId<'i>) -> Result<PresentationAttrId, ()> {
                macro_rules! property_id_filtered {
                    ($_attr:ident) => { PropertyId::$_attr };
                    ($_attr:ident ($_vp:ty)) => { PropertyId::$_attr (_) };
                    ($_attr:ident / false $_name:literal) => { PropertyId::Custom(CustomPropertyName::Unknown(_)) };
                }
                macro_rules! property_id_if {
                    () => { true };
                    (false $_name:literal) => { value == &PropertyId::Custom(CustomPropertyName::Unknown($_name.into())) };
                }
                match value {
                    $(
                        property_id_filtered!($attr $(($vp))? $(/ $is_matching_property_id $name)?) if property_id_if!($($is_matching_property_id $name)?) => Ok(PresentationAttrId::$attr),
                    )+
                    _ => Err(()),
                }
            }
        }

        impl<'i> TryInto<PropertyId<'i>> for &PresentationAttrId {
            type Error = ();

            fn try_into(self) -> Result<PropertyId<'i>, ()> {
                macro_rules! try_property_id {
                    ($_attr:ident) => { Ok(PropertyId::$_attr) };
                    ($_attr:ident($_vp:ty)) => { Ok(PropertyId::$_attr(<$_vp>::None)) };
                    ($_attr:ident / false $_name:literal) => { match PropertyId::from($_name) {
                        PropertyId::Custom(_) => Err(()),
                        id => Ok(id),
                    } };
                }
                match self {
                    $(
                        PresentationAttrId::$attr => try_property_id!($attr $(($vp))? $(/ $is_matching_property_id $name)?),
                    )+
                }
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        /// A presentation attribute
        pub enum PresentationAttr<'i> {
            $(
                #[doc=concat!("The `", $name, "` attribute.")]
                $attr($type),
            )+
        }

        impl PresentationAttr<'_> {
            fn id(&self) -> PresentationAttrId {
                match self {
                    $(Self::$attr(_) => PresentationAttrId::$attr,)+
                }
            }
        }

        impl<'i> TryFrom<&Attr<'i>> for PresentationAttr<'i> {
            type Error = ();

            fn try_from(value: &Attr<'i>) -> Result<Self, Self::Error> {
                match value {
                    $(Attr::$attr(value) => Ok(Self::$attr(value.clone())),)+
                    _ => Err(()),
                }
            }
        }

        impl<'i> Into<Attr<'i>> for PresentationAttr<'i> {
            fn into(self) -> Attr<'i> {
                match self {
                    $(Self::$attr(value) => Attr::$attr(value),)+
                }
            }
        }
    };
}

define_presentation_attrs! {
    "alignment-baseline": AlignmentBaseline(AlignmentBaseline) / false,
    "baseline-shift": BaselineShift(Inheritable<BaselineShift>) / false,
    "clip": Clip(Inheritable<Clip>) / false,
    "clip-path": ClipPath(ClipPath<'i>, VendorPrefix),
    "clip-rule": ClipRule(FillRule),
    "color": Color(Color),
    "color-interpolation": ColorInterpolation(ColorInterpolation),
    "color-interpolation-filters": ColorInterpolationFilters(ColorInterpolation),
    "color-profile": ColorProfile(Inheritable<ColorProfile<'i>>) / false,
    "color-rendering": ColorRendering(Rendering),
    "cursor": Cursor(Cursor<'i>),
    "direction": Direction(Direction),
    "display": Display(Display),
    "dominant-baseline": DominantBaseline(DominantBaseline) / false,
    "enable-background": EnableBackground(Inheritable<EnableBackground>) / false,
    "fill": Fill(Paint<'i>) / false,
    "fill-opacity": FillOpacity(Opacity),
    "fill-rule": FillRule(FillRule),
    "filter": Filter(FilterList<'i>, VendorPrefix),
    "flood-color": FloodColor(Color) / false,
    "flood-opacity": FloodOpacity(Opacity) / false,
    "font": Font(Font<'i>),
    "font-family": FontFamily(FontFamily<'i>),
    "font-size-adjust": FontSizeAdjust(Number) / false,
    "font-size": FontSize(FontSize),
    "font-stretch": FontStretch(FontStretch),
    "font-style": FontStyle(FontStyle),
    "font-variant": FontVariant(FontVariant) / false,
    "font-weight": FontWeight(FontWeight),
    "glyph-orientation-horizontal": GlyphOrientationHorizontal(Angle) / false,
    "glyph-orientation-vertical": GlyphOrientationVertical(Angle) / false,
    "image-rendering": ImageRendering(Rendering),
    "kerning": Kerning(Length) / false,
    "letter-spacing": LetterSpacing(Length),
    "lighting-color": LightingColor(Color) / false,
    "marker": Marker(Marker<'i>),
    "marker-end": MarkerEnd(Marker<'i>),
    "marker-mid": MarkerMid(Marker<'i>),
    "marker-start": MarkerStart(Marker<'i>),
    "mask": Mask(ListOf<Mask<'i>, Comma>, VendorPrefix),
    "opacity": Opacity(Opacity),
    "overflow": Overflow(Overflow),
    "paint-order": PaintOrder(PaintOrder) / false,
    "pointer-events": PointerEvents(PointerEvents) / false,
    "shape-rendering": ShapeRendering(ShapeRendering),
    "stop-color": StopColor(Color) / false,
    "stop-opacity": StopOpacity(Opacity) / false,
    "stroke": Stroke(Paint<'i>) (inheritable: true),
    "stroke-dasharray": StrokeDasharray(StrokeDasharray),
    "stroke-dashoffset": StrokeDashoffset(LengthPercentage),
    "stroke-linecap": StrokeLinecap(StrokeLinecap),
    "stroke-linejoin": StrokeLinejoin(StrokeLinejoin),
    "stroke-miterlimit": StrokeMiterlimit(Number),
    "stroke-opacity": StrokeOpacity(Opacity),
    "stroke-width": StrokeWidth(LengthPercentage),
    "text-anchor": TextAnchor(TextAnchor) (inheritable: true) / false,
    "text-decoration": TextDecoration(TextDecoration, VendorPrefix),
    "text-rendering": TextRendering(Rendering) (inheritable: true),
    "transform-origin": TransformOrigin(Position, VendorPrefix),
    "unicode-bidi": UnicodeBidi(UnicodeBidi),
    "vector-effect": VectorEffect(VectorEffect) / false,
    "visibility": Visibility(Visibility) (inheritable: true),
    "word-spacing": WordSpacing(Spacing) (inheritable: true),
    "writing-mode": WritingMode(WritingMode) (inheritable: true) / false,
}

#[derive(Default, Debug)]
/// A mode in which a style can be applied to an element
pub enum Mode {
    #[default]
    /// The application of a style based on an attribute, style attribute, or static stylesheet selector
    Static,
    /// The application of a style based on an at-rule or psuedo-class
    Dynamic,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
/// A style id for a style applied to an element
pub enum Id<'i> {
    /// A CSS property id
    CSS(PropertyId<'i>),
    /// A presentation attribute id
    Attr(PresentationAttrId),
}

#[derive(Debug, Clone)]
/// A style from either an attribute, style attribute, or static stylesheet selector
pub enum Static<'i> {
    /// A style from a style attribute or static stylesheet selector
    Css(Property<'i>),
    /// A style from an attribute
    Attr(PresentationAttr<'i>),
}

#[derive(Debug, Clone)]
/// A style that can be applied to an element, through either an attribute, style attribute, or stylesheet
pub enum Style<'i> {
    /// The style is declared directly through an attribute, style attribute, or static stylesheet selector
    Static(Static<'i>),
    /// The style is declared within a pseudo-class or at-rule
    Dynamic(Property<'i>),
}

#[derive(Default, Debug)]
/// A collection of the different styles and how they're applied to a given element
pub struct ComputedStyles<'input> {
    /// Inherited styles (e.g. `p`'s color from `<div style="color: red;"><p /></div>`)
    pub inherited: HashMap<Id<'input>, Style<'input>>,
    /// Styles (e.g. `<style>p { color: red }</style>`)
    pub declarations: HashMap<PropertyId<'input>, (u32, Style<'input>)>,
    /// Presentation attributes (e.g. `<p color="red" />`)
    pub attr: HashMap<PresentationAttrId, Style<'input>>,
    /// Inline styles (e.g. `<p style="color: red;" />`)
    pub inline: HashMap<PropertyId<'input>, Style<'input>>,
    /// Important styles (e.g. `<style>p { color: red !important; }</style>`)
    pub important_declarations: HashMap<PropertyId<'input>, (u32, Style<'input>)>,
    /// Important inline styles (e.g. `<p style="color: red !important;" />`)
    pub inline_important: HashMap<PropertyId<'input>, Style<'input>>,
}

/// Gathers stylesheet declarations from the document
///
/// # Panics
/// If the internal selector is invalid
pub fn root<'arena, 'a>(root: &Element<'arena>) -> impl Iterator<Item = CssRule<'arena>> {
    root.breadth_first()
        .filter_map(|node| match node.node_data {
            crate::node::NodeData::Style(ref style) => Some(style),
            _ => None,
        })
        .flat_map(|css_rule_list| css_rule_list.0.iter())
        .cloned()
}

#[derive(Debug)]
/// Contains a collection of style data associated with an element
pub struct ElementData<'arena> {
    inline_style: Option<crate::attribute::data::core::Style<'arena>>,
    presentation_attrs: Vec<Attr<'arena>>,
}

impl<'arena> Default for ElementData<'arena> {
    fn default() -> Self {
        Self {
            inline_style: None,
            presentation_attrs: vec![],
        }
    }
}

impl<'arena> ElementData<'arena> {
    /// Create's a map of associated data for every descendant of the given element
    pub fn new(root: &Element<'arena>) -> HashMap<Element<'arena>, Self> {
        let mut styles = HashMap::new();
        for element in root.breadth_first() {
            let mut inline_style = None;
            let presentation_attrs = element
                .attributes()
                .into_iter()
                .filter(|a| {
                    if a.name()
                        .attribute_group()
                        .contains(AttributeGroup::Presentation)
                    {
                        true
                    } else {
                        if let Attr::Style(style) = &**a {
                            inline_style = Some(style.clone());
                        }
                        false
                    }
                })
                .map(|a| a.clone())
                .collect();
            styles.insert(
                element,
                ElementData {
                    inline_style,
                    presentation_attrs,
                },
            );
        }

        styles
    }
}

impl<'arena> ComputedStyles<'arena> {
    /// Include all sources of styles
    pub fn with_all(
        self,
        element: &Element<'arena>,
        styles: &Option<CssRuleList<'arena>>,
        element_styles: &HashMap<Element<'arena>, ElementData<'arena>>,
    ) -> ComputedStyles<'arena> {
        self.with_inline_style(element, element_styles)
            .with_attribute(element, element_styles)
            .with_style(element, styles)
            .with_inherited(element, styles, element_styles)
    }

    /// Include the computed styles of a parent element
    pub fn with_inherited(
        mut self,
        element: &Element<'arena>,
        styles: &Option<CssRuleList<'arena>>,
        element_styles: &HashMap<Element<'arena>, ElementData<'arena>>,
    ) -> ComputedStyles<'arena> {
        let Some(parent) = Element::parent_element(element) else {
            return self;
        };
        let parent_styles = ComputedStyles::default().with_all(&parent, styles, element_styles);
        self.inherited.extend(parent_styles.inherited);
        self.inherited.extend(
            parent_styles
                .attr
                .into_iter()
                .map(|(id, value)| (Id::Attr(id), value)),
        );
        self.inherited.extend(
            parent_styles
                .declarations
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value.1)),
        );
        self.inherited.extend(
            parent_styles
                .inline
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value)),
        );
        self.inherited.extend(
            parent_styles
                .important_declarations
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value.1)),
        );
        self.inherited.extend(
            parent_styles
                .inline_important
                .into_iter()
                .map(|(id, value)| (Id::CSS(id), value)),
        );
        self
    }

    /// Include styles from the `style` attribute
    pub fn with_style(
        mut self,
        element: &Element<'arena>,
        styles: &Option<CssRuleList<'arena>>,
    ) -> ComputedStyles<'arena> {
        let Some(styles) = styles.as_ref() else {
            return self;
        };
        styles
            .0
            .iter()
            .for_each(|s| self.with_nested_style(element, s, "", 0, &Mode::Static));
        self
    }

    /// Include a style within a style scope
    fn with_nested_style(
        &mut self,
        element: &Element<'arena>,
        style: &rules::CssRule<'arena>,
        selector: &str,
        specificity: u32,
        mode: &Mode,
    ) {
        match style {
            rules::CssRule::Style(r) => r.selectors.0.iter().for_each(|s| {
                cfg_if! {
                    if #[cfg(feature = "selectors")] {
                        use crate::selectors::{SelectElement, Selector};
                        let Ok(this_selector) = s.to_css_string(printer::PrinterOptions::default()) else {
                            return;
                        };
                        let selector = format!("{selector}{this_selector}");
                        let Ok(select) = Selector::new(&selector) else {
                            return;
                        };
                        if !select.matches_naive(&SelectElement::new(element.clone())) {
                            return;
                        }
                        let specificity = specificity + s.specificity();
                        self.add_declarations(&r.declarations, specificity, mode);
                    } else {
                        return;
                    }
                }
            }),
            rules::CssRule::Container(rules::container::ContainerRule { rules, .. })
            | rules::CssRule::Media(rules::media::MediaRule { rules, .. }) => {
                rules.0.iter().for_each(|r| {
                    self.with_nested_style(element, r, selector, specificity, &Mode::Dynamic);
                });
            }
            _ => {}
        }
    }

    /// Include styles from a presentable attribute
    fn with_attribute(
        self,
        element: &Element<'arena>,
        element_styles: &HashMap<Element<'arena>, ElementData<'arena>>,
    ) -> ComputedStyles<'arena> {
        let Some(element_styles) = element_styles.get(element) else {
            return self;
        };
        let attr = element_styles
            .presentation_attrs
            .iter()
            .filter_map(|attr| {
                let value = PresentationAttr::try_from(attr).ok()?;
                Some((value.id(), Mode::Static.style(Static::Attr(value))))
            })
            .collect();
        ComputedStyles { attr, ..self }
    }

    fn with_inline_style(
        self,
        element: &Element<'arena>,
        element_styles: &HashMap<Element<'arena>, ElementData<'arena>>,
    ) -> ComputedStyles<'arena> {
        let Some(element_styles) = element_styles.get(element) else {
            return self;
        };
        let Some(style) = element_styles.inline_style.as_ref() else {
            return self;
        };
        let mut inline = HashMap::new();
        let mut inline_important = HashMap::new();
        style.declarations.iter().for_each(|s| {
            inline.insert(s.property_id(), Mode::Static.style(Static::Css(s.clone())));
        });
        style.important_declarations.iter().for_each(|s| {
            inline_important.insert(s.property_id(), Mode::Static.style(Static::Css(s.clone())));
        });

        ComputedStyles {
            inline,
            inline_important,
            ..self
        }
    }

    /// Get's a style by id, agnostic of whether it's a presentation attr or css id
    pub fn get(&'arena self, id: &Id<'arena>) -> Option<&'arena Style<'arena>> {
        self.get_important(id).or_else(|| self.get_unimportant(id))
    }

    /// Returns whether the given id is resolved by the computed styles.
    pub fn has(&'arena self, id: &Id<'arena>) -> bool {
        self.get(id).is_some()
    }

    /// Gets the resolved style from a presentation attribute id.
    pub fn get_with_attr(&'arena self, id: PresentationAttrId) -> Option<&'arena Style<'arena>> {
        let id = Id::Attr(id);
        if let Some(value) = self.get_important(&id) {
            Some(value)
        } else if let Some(value) = self.get_unimportant(&id) {
            Some(value)
        } else {
            None
        }
    }

    fn get_important(&'arena self, id: &Id<'arena>) -> Option<&'arena Style<'arena>> {
        match id {
            Id::CSS(id) => {
                if let Some(value) = self.inline_important.get(id) {
                    Some(value)
                } else if let Some((_, value)) = self.important_declarations.get(id) {
                    Some(value)
                } else {
                    None
                }
            }
            Id::Attr(id) => self.get_important(&Id::CSS(id.try_into().ok()?)),
        }
    }

    fn get_unimportant(&'arena self, id: &Id<'arena>) -> Option<&'arena Style<'arena>> {
        match id {
            Id::CSS(id) => {
                if let Some(value) = self.inline.get(id) {
                    return Some(value);
                }
                if let Ok(id) = PresentationAttrId::try_from(id) {
                    if let Some(value) = self.attr.get(&id) {
                        return Some(value);
                    }
                }
                if let Some((_, value)) = self.declarations.get(id) {
                    return Some(value);
                }
            }
            Id::Attr(id) => {
                if let Some(value) = self.attr.get(id) {
                    return Some(value);
                }
            }
        }
        self.inherited.get(id)
    }

    /// Gets the resolved static style from an id.
    pub fn get_static<'a>(&'arena self, id: &'a Id<'a>) -> Option<&'arena Static<'arena>>
    where
        'a: 'arena,
    {
        match self.get(id) {
            Some(Style::Static(value)) => Some(value),
            _ => None,
        }
    }

    /// Gets the collection of all the resolved styles applied to the element.
    pub fn computed(&'arena self) -> HashMap<Id<'arena>, &'arena Style<'arena>> {
        let mut result = HashMap::new();
        let map = |s: &'arena (u32, Style<'arena>)| &s.1;
        let mut insert = |s: &'arena Style<'arena>| {
            result.insert(s.id(), s);
        };
        self.attr.values().for_each(&mut insert);
        self.declarations.values().map(map).for_each(&mut insert);
        self.inline.values().for_each(&mut insert);
        self.important_declarations
            .values()
            .map(map)
            .for_each(&mut insert);
        self.inline_important.values().for_each(insert);
        result
    }

    /// Consumed the [`ComputedStyles`] and creates a collection of all the resolved
    /// styles applied to the element.
    pub fn into_computed(self) -> HashMap<Id<'arena>, Style<'arena>> {
        let mut result = HashMap::new();
        let map = |s: (u32, Style<'arena>)| s.1;
        let mut insert = |s: Style<'arena>| {
            result.insert(s.id(), s);
        };
        self.inherited.into_values().for_each(&mut insert);
        self.attr.into_values().for_each(&mut insert);
        self.declarations
            .into_values()
            .map(map)
            .for_each(&mut insert);
        self.inline.into_values().for_each(&mut insert);
        self.important_declarations
            .into_values()
            .map(map)
            .for_each(&mut insert);
        self.inline_important.into_values().for_each(insert);
        result
    }

    fn add_declarations(
        &mut self,
        declarations: &declaration::DeclarationBlock<'arena>,
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
        record: &mut HashMap<PropertyId<'arena>, (u32, Style<'arena>)>,
        declarations: &[properties::Property<'arena>],
        specificity: u32,
        mode: &Mode,
    ) {
        for d in declarations {
            let id = d.property_id();
            record.insert(id, (specificity, mode.style(Static::Css(d.clone()))));
        }
    }
}

#[macro_export]
/// Creates a macro called `get_computed_styles` that can be used to get the effective
/// style from [`ComputedStyles`] based on a lightningcss property or presentation attribute
macro_rules! get_computed_styles_factory {
    ($item:ident) => {
        macro_rules! get_computed_styles {
            // NOTE: Branches should be identical, apart from $vp
            ($ident:ident) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident)
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident))
                    .or_else(|| $item.inline.get(&PropertyId::$ident))
                    .or_else(|| $item.declarations.get(&PropertyId::$ident).map(|p| &p.1))
                    .or_else(|| $item.attr.get(&PresentationAttrId::$ident))
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident)))
                    .or_else(|| $item.inherited.get(&Id::Attr(PresentationAttrId::$ident)))
            };
            ($ident:ident ( $vp:expr )) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident($vp))
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident($vp)))
                    .or_else(|| $item.inline.get(&PropertyId::$ident($vp)))
                    .or_else(|| {
                        $item
                            .declarations
                            .get(&PropertyId::$ident($vp))
                            .map(|p| &p.1)
                    })
                    .or_else(|| $item.attr.get(&PresentationAttrId::$ident))
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident($vp))))
                    .or_else(|| $item.inherited.get(&Id::Attr(PresentationAttrId::$ident)))
            };
        }
    };
}

#[macro_export]
/// Creates a macro called `get_computed_property` that can be used to get the effective
/// style from [`ComputedStyles`] based on a lightningcss property
macro_rules! get_computed_property_factory {
    ($item:ident) => {
        macro_rules! get_computed_property {
            // NOTE: Two branches should be identical, apart from $vp
            ($ident:ident) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident)
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident))
                    .or_else(|| $item.inline.get(&PropertyId::$ident))
                    .or_else(|| $item.declarations.get(&PropertyId::$ident).map(|p| &p.1))
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident)))
            };
            ($ident:ident ( $vp:expr )) => {
                $item
                    .important_declarations
                    .get(&PropertyId::$ident($vp))
                    .map(|p| &p.1)
                    .or_else(|| $item.inline_important.get(&PropertyId::$ident($vp)))
                    .or_else(|| $item.inline.get(&PropertyId::$ident($vp)))
                    .or_else(|| {
                        $item
                            .declarations
                            .get(&PropertyId::$ident($vp))
                            .map(|p| &p.1)
                    })
                    .or_else(|| $item.inherited.get(&Id::CSS(PropertyId::$ident($vp))))
            };
        }
    };
}

impl<'i> Static<'i> {
    /// Returns the id of a style
    pub fn id(&self) -> Id<'i> {
        match self {
            Self::Css(property) => Id::CSS(property.property_id()),
            Self::Attr(attr) => Id::Attr(attr.id()),
        }
    }
}

impl<'i> Style<'i> {
    /// Returns the style as a static style
    pub fn inner(&self) -> Static<'i> {
        match self {
            Self::Static(v) => v.clone(),
            Self::Dynamic(v) => Static::Css(v.clone()),
        }
    }

    /// Returns the id of a style
    pub fn id(&self) -> Id<'i> {
        self.inner().id()
    }

    /// Returns the mode in which the styles are applied to an element. i.e. static or dynamic
    pub fn mode(&self) -> Mode {
        match self {
            Self::Static(_) => Mode::Static,
            Self::Dynamic(_) => Mode::Dynamic,
        }
    }

    /// Returns whether the style was part of an attribute or non-dynamic selector
    pub fn is_static(&self) -> bool {
        self.mode().is_static()
    }

    /// Returns whether the style was part of an at-rule or pseudo-class
    pub fn is_dynamic(&self) -> bool {
        self.mode().is_dynamic()
    }

    /// Returns whether the style wasn't able to be parsed
    pub fn is_unparsed(&self) -> bool {
        match self {
            Self::Static(style) => match style {
                Static::Css(css) => matches!(css, Property::Unparsed(_)),
                Static::Attr(_) => false,
            },
            Self::Dynamic(css) => matches!(css, Property::Unparsed(_)),
        }
    }

    /// Gets the presentation attribute representation if the style is sourced from an attribute
    pub fn presentation_attr(&self) -> Option<PresentationAttr<'i>> {
        match self.inner() {
            Static::Attr(attr) => Some(attr),
            Static::Css(_) => None,
        }
    }

    /// Gets the lightningcss representation if the style is sourced from a stylesheet
    pub fn property(&self) -> Option<Property<'i>> {
        match self.inner() {
            Static::Css(css) => Some(css),
            Static::Attr(_) => None,
        }
    }
}

impl Mode {
    /// # Panics
    /// If attempting to assign attribute to dynamic style
    pub fn style<'i>(&self, style: Static<'i>) -> Style<'i> {
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
