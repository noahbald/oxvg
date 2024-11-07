use std::{borrow::Borrow, collections::BTreeMap, rc::Rc};

use lazy_static::lazy_static;
use lightningcss::{
    declaration,
    printer::{self, PrinterOptions},
    properties::{self, transform::TransformList},
    rules::{self},
    stylesheet::{self, ParserOptions},
    traits::ToCss,
    values,
    vendor_prefix::{self, VendorPrefix},
};
use markup5ever::{local_name, Attribute, LocalName};
use oxvg_selectors::{Element, Selector};
use rcdom::NodeData;

#[derive(Debug)]
pub enum SVGPaint {
    Url,
    Color(values::color::CssColor),
    ContextFill,
    ContextStroke,
    None,
}

/// Relevant css properties in an owned variant
#[derive(Debug)]
pub enum SVGStyle {
    MarkerMid(Option<String>),
    Stroke(SVGPaint),
    StrokeDasharray(properties::svg::StrokeDasharray),
    StrokeDashoffset(values::length::LengthPercentage),
    StrokeLinecap(properties::svg::StrokeLinecap),
    StrokeLinejoin(properties::svg::StrokeLinejoin),
    StrokeWidth(values::percentage::DimensionPercentage<values::length::LengthValue>),
    Transform(
        properties::transform::TransformList,
        vendor_prefix::VendorPrefix,
    ),
    GradientTransform(properties::transform::TransformList),
    PatternTransform(properties::transform::TransformList),
    /// The matched style isn't relevant for SVG optimisation
    Unsupported,
}

#[derive(Ord, Eq, PartialEq, PartialOrd, Debug)]
pub enum SVGStyleID {
    MarkerMid,
    Stroke,
    StrokeDasharray,
    StrokeDashoffset,
    StrokeLinecap,
    StrokeLinejoin,
    StrokeWidth,
    Transform,
    GradientTransform,
    PatternTransform,
    Unsupported,
    Unspecified,
}

#[derive(Debug)]
pub enum Style {
    /// The style is declared directly through an attribute, style attribute, or stylesheet
    Static(SVGStyle),
    /// The style is declared within a pseudo-class or at-rule
    Dyanmic(SVGStyle),
}

#[derive(Default, Debug)]
pub enum StyleMode {
    #[default]
    Static,
    Dynamic,
}

#[derive(Default, Debug)]
pub struct ComputedStyles {
    pub inherited: BTreeMap<SVGStyleID, Style>,
    pub declarations: BTreeMap<SVGStyleID, (u32, Style)>,
    pub attr: BTreeMap<SVGStyleID, Style>,
    pub inline: BTreeMap<SVGStyleID, Style>,
    pub important_declarations: BTreeMap<SVGStyleID, (u32, Style)>,
    pub inline_important: BTreeMap<SVGStyleID, Style>,
}

/// Gathers stylesheet declarations from the document
///
/// # Panics
/// If the internal selector is invalid
pub fn root_style(root: &Element) -> String {
    root.select("style")
        .expect("`style` should be a valid selector")
        .map(|s| {
            s.text_content()
                .map(|s| s.borrow().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl ComputedStyles {
    /// Include all sources of styles
    pub fn with_all(&mut self, node: &Rc<rcdom::Node>, styles: &[rules::CssRule]) {
        self.with_inherited(node, styles);
        self.with_style(node, styles);
        self.with_attribute(node);
        self.with_inline_style(node);
    }

    /// Include the computed styles of a parent element
    pub fn with_inherited(&mut self, node: &Rc<rcdom::Node>, styles: &[rules::CssRule]) {
        let element = Element::new(node.clone());
        let Some(parent) = element.get_parent() else {
            return;
        };
        let mut inherited = ComputedStyles::default();
        inherited.with_all(&parent.node, styles);
        self.inherited = inherited.into_computed();
    }

    /// Include styles from the `style` attribute
    pub fn with_style(&mut self, node: &Rc<rcdom::Node>, styles: &[rules::CssRule]) {
        styles
            .iter()
            .for_each(|s| self.with_nested_style(node, s, "", 0, &StyleMode::Static));
    }

    /// Include a style within a style scope
    fn with_nested_style(
        &mut self,
        node: &Rc<rcdom::Node>,
        style: &rules::CssRule,
        selector: &str,
        specificity: u32,
        mode: &StyleMode,
    ) {
        match style {
            rules::CssRule::Style(r) => r.selectors.0.iter().for_each(|s| {
                let Ok(this_selector) = s.to_css_string(printer::PrinterOptions::default()) else {
                    return;
                };
                let selector = format!("{selector}{this_selector}");
                let Ok(select) = Selector::try_from(selector.as_str()) else {
                    return;
                };
                let element = Element::new(node.clone());
                if !select.matches_naive(&element) {
                    return;
                };
                let specificity = specificity + s.specificity();
                self.add_declarations(&r.declarations, specificity, mode);
            }),
            rules::CssRule::Container(rules::container::ContainerRule { rules, .. })
            | rules::CssRule::Media(rules::media::MediaRule { rules, .. }) => {
                rules.0.iter().for_each(|r| {
                    self.with_nested_style(node, r, selector, specificity, &StyleMode::Dynamic);
                });
            }
            _ => {}
        }
    }

    /// Include styles from a presentable attribute
    fn with_attribute(&mut self, node: &Rc<rcdom::Node>) {
        let node: &rcdom::Node = node.borrow();
        let NodeData::Element { ref attrs, .. } = node.data else {
            return;
        };
        attrs.borrow().iter().for_each(|a| {
            let Ok(style) = SVGStyle::try_from(a) else {
                return;
            };
            self.attr.insert(style.id(), StyleMode::Static.style(style));
        });
    }

    pub fn with_inline_style(&mut self, node: &Rc<rcdom::Node>) {
        let element = Element::new(node.clone());
        let Some(style) = element.get_attr(&local_name!("style")) else {
            return;
        };
        let Some(style) =
            stylesheet::StyleAttribute::parse(&style.value, stylesheet::ParserOptions::default())
                .ok()
        else {
            return;
        };
        style
            .declarations
            .declarations
            .iter()
            .map(SVGStyle::from)
            .for_each(|s| {
                self.inline.insert(s.id(), StyleMode::Static.style(s));
            });
        style
            .declarations
            .important_declarations
            .iter()
            .map(SVGStyle::from)
            .for_each(|s| {
                self.inline_important
                    .insert(s.id(), StyleMode::Static.style(s));
            });
    }

    pub fn get(&self, id: &SVGStyleID) -> Option<&Style> {
        if let Some(value) = self.get_important(id) {
            return Some(value);
        } else if let Some(value) = self.get_unimportant(id) {
            return Some(value);
        }
        None
    }

    pub fn get_string(&self, id: &SVGStyleID) -> Option<(StyleMode, String)> {
        let mut important = false;
        let value = if let Some(value) = self.get_important(id) {
            important = true;
            value
        } else if let Some(value) = self.get_unimportant(id) {
            value
        } else {
            return None;
        };
        let string = value.to_css_string(important)?;
        Some((value.mode(), string))
    }

    fn get_important(&self, id: &SVGStyleID) -> Option<&Style> {
        if let Some(value) = self.inline_important.get(id) {
            return Some(value);
        } else if let Some((_, value)) = self.important_declarations.get(id) {
            return Some(value);
        }
        None
    }

    fn get_unimportant(&self, id: &SVGStyleID) -> Option<&Style> {
        if let Some(value) = self.inline.get(id) {
            return Some(value);
        } else if let Some(value) = self.attr.get(id) {
            return Some(value);
        } else if let Some((_, value)) = self.declarations.get(id) {
            return Some(value);
        } else if let Some(value) = self.inherited.get(id) {
            return Some(value);
        }
        None
    }

    pub fn get_static(&self, id: &SVGStyleID) -> Option<&SVGStyle> {
        match self.get(id) {
            Some(Style::Static(value)) => Some(value),
            _ => None,
        }
    }

    pub fn computed<'a>(&'a self) -> BTreeMap<SVGStyleID, &'a Style> {
        let mut result = BTreeMap::new();
        let map = |s: &'a (u32, Style)| &s.1;
        let mut insert = |s: &'a Style| {
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

    pub fn into_computed(self) -> BTreeMap<SVGStyleID, Style> {
        let mut result = BTreeMap::new();
        let map = |s: (u32, Style)| s.1;
        let mut insert = |s: Style| {
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
        declarations: &declaration::DeclarationBlock,
        specificity: u32,
        mode: &StyleMode,
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
        record: &mut BTreeMap<SVGStyleID, (u32, Style)>,
        declarations: &[properties::Property],
        specificity: u32,
        mode: &StyleMode,
    ) {
        for d in declarations {
            let decl = SVGStyle::from(d);
            let id = decl.id();
            record.insert(id, (specificity, mode.style(decl)));
        }
    }
}

impl SVGStyle {
    fn id(&self) -> SVGStyleID {
        match self {
            Self::MarkerMid(_) => SVGStyleID::MarkerMid,
            Self::Stroke(_) => SVGStyleID::Stroke,
            Self::StrokeDasharray(_) => SVGStyleID::StrokeDasharray,
            Self::StrokeDashoffset(_) => SVGStyleID::StrokeDashoffset,
            Self::StrokeLinecap(_) => SVGStyleID::StrokeLinecap,
            Self::StrokeLinejoin(_) => SVGStyleID::StrokeLinejoin,
            Self::StrokeWidth(_) => SVGStyleID::StrokeWidth,
            Self::Transform(_, _) => SVGStyleID::Transform,
            Self::GradientTransform(_) => SVGStyleID::GradientTransform,
            Self::PatternTransform(_) => SVGStyleID::PatternTransform,
            Self::Unsupported => SVGStyleID::Unsupported,
        }
    }

    pub fn to_css_string(&self, important: bool) -> Option<String> {
        let property = match self {
            Self::Transform(list, prefix) => properties::Property::Transform(list.clone(), *prefix),
            _ => return None,
        };
        property
            .to_css_string(important, PrinterOptions::default())
            .ok()
    }
}

impl TryFrom<&Attribute> for SVGStyle {
    type Error = ();

    fn try_from(value: &Attribute) -> Result<Self, Self::Error> {
        let style_id = SVGStyleID::from(&value.name.local);
        let Some(property_id) = style_id.property_id() else {
            return Err(());
        };
        let mut value = value.value.to_string();
        if matches!(property_id, properties::PropertyId::Transform(_)) {
            let v = ROTATE_LONG.replace_all(&value, |caps: &regex::Captures| {
                let original = format!("rotate({} {} {})", &caps["r"], &caps["x"], &caps["y"]);
                let Ok(deg) = caps["r"].parse::<f64>() else {
                    log::debug!("r failed: {}", &caps["r"]);
                    return original;
                };
                let Ok(x) = caps["x"].parse::<f64>() else {
                    log::debug!("x failed: {}", &caps["x"]);
                    return original;
                };
                let Ok(y) = caps["y"].parse::<f64>() else {
                    log::debug!("y failed: {}", &caps["y"]);
                    return original;
                };
                let rad = deg.to_radians();
                let cos = rad.cos();
                let sin = rad.sin();
                format!(
                    "matrix({cos} {sin} {} {cos} {} {})",
                    -sin,
                    (1.0 - cos) * x + sin * y,
                    (1.0 - cos) * y - sin * x
                )
            });
            let v = LIST_SEP_SPACE.replace_all(&v, "$a, ");
            let v = LIST_SEP_FIX.replace_all(&v, ")");
            let v = ROTATE.replace_all(&v, |caps: &regex::Captures| {
                format!("{}({}deg", &caps["f"], &caps["v"])
            });
            let v = FUNC_SEP.replace_all(&v, ") ");
            log::debug!("tried making transform compatible: {value} -> {v}");
            value = v.to_string();
        }
        let property =
            properties::Property::parse_string(property_id, &value, ParserOptions::default());
        match property {
            Ok(property) => match SVGStyle::from((&property, &style_id)) {
                Self::Unsupported => style_id.empty_style().ok_or(()),
                style => Ok(style),
            },
            Err(e) => {
                log::debug!("failed to parse attribute: {}", e.to_string());
                style_id.empty_style().ok_or(())
            }
        }
    }
}

impl From<&properties::Property<'_>> for SVGStyle {
    fn from(value: &properties::Property<'_>) -> Self {
        Self::from((value, &SVGStyleID::Unspecified))
    }
}

impl From<(&properties::Property<'_>, &SVGStyleID)> for SVGStyle {
    fn from((property, id): (&properties::Property<'_>, &SVGStyleID)) -> Self {
        match property {
            properties::Property::MarkerMid(m) => SVGStyle::MarkerMid(match m {
                lightningcss::properties::svg::Marker::Url(u) => Some(u.url.to_string()),
                lightningcss::properties::svg::Marker::None => None,
            }),
            properties::Property::Stroke(s) => SVGStyle::from(s),
            properties::Property::StrokeDasharray(a) => SVGStyle::StrokeDasharray(a.clone()),
            properties::Property::StrokeDashoffset(l) => SVGStyle::StrokeDashoffset(l.clone()),
            properties::Property::StrokeLinecap(s) => SVGStyle::StrokeLinecap(*s),
            properties::Property::StrokeLinejoin(s) => SVGStyle::StrokeLinejoin(*s),
            properties::Property::StrokeWidth(s) => SVGStyle::StrokeWidth(s.clone()),
            properties::Property::Transform(l, p)
                if matches!(id, SVGStyleID::Transform | SVGStyleID::Unspecified) =>
            {
                SVGStyle::Transform(l.clone(), *p)
            }
            properties::Property::Transform(l, _) if id == &SVGStyleID::GradientTransform => {
                SVGStyle::GradientTransform(l.clone())
            }
            properties::Property::Transform(l, _) if id == &SVGStyleID::PatternTransform => {
                SVGStyle::PatternTransform(l.clone())
            }
            properties::Property::Unparsed(_) => {
                log::debug!("property may be valid but couldn't be parsed");
                SVGStyle::Unsupported
            }
            _ => SVGStyle::Unsupported,
        }
    }
}

impl From<&lightningcss::properties::svg::SVGPaint<'_>> for SVGStyle {
    fn from(value: &lightningcss::properties::svg::SVGPaint) -> Self {
        Self::Stroke(SVGPaint::from(value))
    }
}

impl From<&lightningcss::properties::svg::SVGPaint<'_>> for SVGPaint {
    fn from(value: &lightningcss::properties::svg::SVGPaint<'_>) -> Self {
        match value {
            properties::svg::SVGPaint::Url { .. } => Self::Url,
            properties::svg::SVGPaint::Color(c) => Self::Color(c.clone()),
            properties::svg::SVGPaint::ContextFill => Self::ContextFill,
            properties::svg::SVGPaint::ContextStroke => Self::ContextStroke,
            properties::svg::SVGPaint::None => Self::None,
        }
    }
}

impl From<&LocalName> for SVGStyleID {
    fn from(value: &LocalName) -> Self {
        match *value {
            local_name!("marker-mid") => Self::MarkerMid,
            local_name!("stroke") => Self::Stroke,
            local_name!("stroke-dasharray") => Self::StrokeDasharray,
            local_name!("stroke-dashoffset") => Self::StrokeDashoffset,
            local_name!("stroke-linecap") => Self::StrokeLinecap,
            local_name!("stroke-linejoin") => Self::StrokeLinejoin,
            local_name!("stroke-width") => Self::StrokeWidth,
            local_name!("transform") => Self::Transform,
            local_name!("gradientTransform") => Self::GradientTransform,
            local_name!("patternTransform") => Self::PatternTransform,
            _ => Self::Unsupported,
        }
    }
}

impl SVGStyleID {
    fn property_id(&self) -> Option<properties::PropertyId> {
        let property_id = match self {
            Self::MarkerMid => properties::PropertyId::MarkerMid,
            Self::Stroke => properties::PropertyId::Stroke,
            Self::StrokeDasharray => properties::PropertyId::StrokeDasharray,
            Self::StrokeDashoffset => properties::PropertyId::StrokeDashoffset,
            Self::StrokeLinecap => properties::PropertyId::StrokeLinecap,
            Self::StrokeLinejoin => properties::PropertyId::StrokeLinejoin,
            Self::StrokeWidth => properties::PropertyId::StrokeWidth,
            Self::Transform | Self::GradientTransform | Self::PatternTransform => {
                properties::PropertyId::Transform(vendor_prefix::VendorPrefix::None)
            }
            _ => return None,
        };
        Some(property_id)
    }

    fn empty_style(&self) -> Option<SVGStyle> {
        Some(match self {
            Self::Transform => SVGStyle::Transform(TransformList(vec![]), VendorPrefix::None),
            Self::GradientTransform => SVGStyle::GradientTransform(TransformList(vec![])),
            Self::PatternTransform => SVGStyle::PatternTransform(TransformList(vec![])),
            _ => return None,
        })
    }
}

impl Style {
    pub fn inner(&self) -> &SVGStyle {
        match self {
            Self::Static(v) | Self::Dyanmic(v) => v,
        }
    }

    pub fn id(&self) -> SVGStyleID {
        self.inner().id()
    }

    pub fn mode(&self) -> StyleMode {
        match self {
            Self::Static(_) => StyleMode::Static,
            Self::Dyanmic(_) => StyleMode::Dynamic,
        }
    }

    pub fn is_static(&self) -> bool {
        self.mode().is_static()
    }

    pub fn is_dynamic(&self) -> bool {
        self.mode().is_dynamic()
    }

    pub fn to_css_string(&self, important: bool) -> Option<String> {
        self.inner().to_css_string(important)
    }
}

impl StyleMode {
    fn style(&self, style: SVGStyle) -> Style {
        match self {
            Self::Static => Style::Static(style),
            Self::Dynamic => Style::Dyanmic(style),
        }
    }

    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static)
    }

    pub fn is_dynamic(&self) -> bool {
        !self.is_static()
    }
}

lazy_static! {
    static ref LIST_SEP_SPACE: regex::Regex =
        regex::Regex::new(r"(?<a>-?\d*\.?\d*(e-?)?\d+),?").unwrap();
    static ref LIST_SEP_FIX: regex::Regex = regex::Regex::new(r",\s*\)").unwrap();
    static ref FUNC_SEP: regex::Regex = regex::Regex::new(r"\)[,\s]*").unwrap();
    static ref ROTATE: regex::Regex =
        regex::Regex::new(r"(?<f>rotate|skew|skewX|skewY)\((?<v>\s*[^\s\),]+)").unwrap();
    static ref ROTATE_LONG: regex::Regex = regex::Regex::new(
        r"rotate\((?<r>[\d\.e-]+)[^\d\)]+?(?<x>[\d\.e-]+)[^\d\)]+?(?<y>[\d\.e-]+)\)"
    )
    .unwrap();
}
