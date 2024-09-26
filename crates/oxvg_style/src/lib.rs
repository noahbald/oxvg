use std::{borrow::Borrow, collections::BTreeMap, rc::Rc};

use lightningcss::{declaration, printer, properties, rules, stylesheet, traits::ToCss, values};
use markup5ever::local_name;
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
    StrokeLinecap(properties::svg::StrokeLinecap),
    StrokeLinejoin(properties::svg::StrokeLinejoin),
    /// The matched style isn't relevant for SVG optimisation
    Unsupported,
}

#[derive(Ord, Eq, PartialEq, PartialOrd, Debug)]
pub enum SVGStyleID {
    MarkerMid,
    Stroke,
    SrokeLinecap,
    StrokeLinejoin,
    Unsupported,
}

#[derive(Default, Debug)]
pub struct ComputedStyles {
    pub inherited: BTreeMap<SVGStyleID, SVGStyle>,
    pub declarations: BTreeMap<SVGStyleID, (u32, SVGStyle)>,
    pub inline: BTreeMap<SVGStyleID, SVGStyle>,
    pub important_declarations: BTreeMap<SVGStyleID, (u32, SVGStyle)>,
    pub inline_important: BTreeMap<SVGStyleID, SVGStyle>,
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
    pub fn with_all(&mut self, node: &Rc<rcdom::Node>, styles: &[rules::CssRule]) {
        self.with_inherited(node, styles);
        self.with_style(node, styles);
        self.with_attribute(node);
        self.with_inline_style(node);
    }

    pub fn with_inherited(&mut self, node: &Rc<rcdom::Node>, styles: &[rules::CssRule]) {
        let element = Element::new(node.clone());
        let Some(parent) = element.get_parent() else {
            return;
        };
        let mut inherited = ComputedStyles::default();
        inherited.with_all(&parent.node, styles);
        self.inherited = inherited.into_computed();
    }

    pub fn with_style(&mut self, node: &Rc<rcdom::Node>, styles: &[rules::CssRule]) {
        styles
            .iter()
            .for_each(|s| self.with_nested_style(node, s, "", 0));
    }

    fn with_nested_style(
        &mut self,
        node: &Rc<rcdom::Node>,
        style: &rules::CssRule,
        selector: &str,
        specificity: u32,
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
                self.add_declarations(&r.declarations, specificity);
            }),
            rules::CssRule::Container(rules::container::ContainerRule { rules, .. })
            | rules::CssRule::Media(rules::media::MediaRule { rules, .. }) => {
                rules
                    .0
                    .iter()
                    .for_each(|r| self.with_nested_style(node, r, selector, specificity));
            }
            _ => {}
        }
    }

    fn with_attribute(&mut self, node: &Rc<rcdom::Node>) {
        let node: &rcdom::Node = node.borrow();
        let NodeData::Element { ref attrs, .. } = node.data else {
            return;
        };
        attrs.borrow().iter().for_each(|a| {
            let name = &a.name.local;
            let value = &a.value;
            let style = format!("{name}:{value}");
            let Ok(style) =
                stylesheet::StyleAttribute::parse(&style, stylesheet::ParserOptions::default())
            else {
                return;
            };
            let property = &style.declarations.declarations[0];
            let style = SVGStyle::from(property);
            self.inline.insert(style.id(), style);
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
                self.inline.insert(s.id(), s);
            });
        style
            .declarations
            .important_declarations
            .iter()
            .map(SVGStyle::from)
            .for_each(|s| {
                self.inline_important.insert(s.id(), s);
            });
    }

    pub fn computed<'a>(&'a self) -> BTreeMap<SVGStyleID, &'a SVGStyle> {
        let mut result = BTreeMap::new();
        let map = |s: &'a (u32, SVGStyle)| &s.1;
        let mut insert = |s: &'a SVGStyle| {
            result.insert(s.id(), s);
        };
        self.declarations.values().map(map).for_each(&mut insert);
        self.inline.values().for_each(&mut insert);
        self.important_declarations
            .values()
            .map(map)
            .for_each(&mut insert);
        self.inline_important.values().for_each(insert);
        result
    }

    pub fn into_computed(self) -> BTreeMap<SVGStyleID, SVGStyle> {
        let mut result = BTreeMap::new();
        let map = |s: (u32, SVGStyle)| s.1;
        let mut insert = |s: SVGStyle| {
            result.insert(s.id(), s);
        };
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

    fn add_declarations(&mut self, declarations: &declaration::DeclarationBlock, specificity: u32) {
        Self::set_declarations(
            &mut self.important_declarations,
            &declarations.important_declarations,
            specificity,
        );
        Self::set_declarations(
            &mut self.declarations,
            &declarations.declarations,
            specificity,
        );
    }

    fn set_declarations(
        record: &mut BTreeMap<SVGStyleID, (u32, SVGStyle)>,
        declarations: &[properties::Property],
        specificity: u32,
    ) {
        for d in declarations {
            let decl = SVGStyle::from(d);
            let id = decl.id();
            record.insert(id, (specificity, decl));
        }
    }
}

impl SVGStyle {
    fn id(&self) -> SVGStyleID {
        match self {
            Self::MarkerMid(_) => SVGStyleID::MarkerMid,
            Self::Stroke(_) => SVGStyleID::Stroke,
            Self::StrokeLinecap(_) => SVGStyleID::SrokeLinecap,
            Self::StrokeLinejoin(_) => SVGStyleID::StrokeLinejoin,
            Self::Unsupported => SVGStyleID::Unsupported,
        }
    }
}

impl From<&properties::Property<'_>> for SVGStyle {
    fn from(value: &properties::Property) -> Self {
        match value {
            properties::Property::MarkerMid(m) => SVGStyle::MarkerMid(match m {
                lightningcss::properties::svg::Marker::Url(u) => Some(u.url.to_string()),
                lightningcss::properties::svg::Marker::None => None,
            }),
            properties::Property::Stroke(s) => SVGStyle::from(s),
            properties::Property::StrokeLinecap(s) => SVGStyle::StrokeLinecap(*s),
            properties::Property::StrokeLinejoin(s) => SVGStyle::StrokeLinejoin(*s),
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
