use std::{
    cell::{self, RefCell},
    collections::{HashMap, HashSet},
    fmt::Write,
    ops::Range,
};

use lightningcss::visitor::Visit as _;
use oxvg_ast::{
    element::Element,
    node::Ranges,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    atom::Atom,
    attribute::{Attr, AttrId},
    element::ElementId,
    name::{Prefix, QualName},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::Error;

mod no_default_attributes;
mod no_deprecated;
mod no_invalid_attributes;
mod no_unknown_attributes;
mod no_unknown_elements;
mod no_unused_ids;
mod no_unused_xmlns;
mod no_xlink;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
/// What is the severity of the reported error.
pub enum Severity {
    /// The error emits no message
    Off,
    /// The error emits a message as a warning
    Warn,
    #[default]
    /// The error emits a message as an error
    Error,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// A set of rules to assert against a document.
///
/// The [`Severity`] provided for each rule determines the display of each attribute
/// by the [`crate::error::Report`].
pub struct Rules {
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using elements that do not belong to a document's content model
    pub no_unknown_elements: Severity,
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using attributes that do not belong to a known element's content model
    pub no_unknown_attributes: Severity,
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using deprecated elements and attributes
    pub no_deprecated: Severity,
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using attribute values that can be omitted
    pub no_default_attributes: Severity,
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using xlink attributes
    pub no_x_link: Severity,
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using id values that are not referenced by the document
    pub no_unused_ids: Severity,
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using xmlns attributes that are not referenced by the document
    pub no_unused_xmlns: Severity,
    #[cfg_attr(feature = "serde", serde(default = "Severity::off"))]
    /// Disallow using attributes that do not fit the expected content-type
    pub no_invalid_attributes: Severity,
}

type NamespaceStack<'input> = Vec<HashSet<(Option<Atom<'input>>, Atom<'input>, bool)>>;
struct Reporter<'o, 'input> {
    rules: &'o Rules,
    reports: RefCell<Vec<Error<'input>>>,
    ids: RefCell<HashMap<Atom<'input>, Option<Ranges>>>,
    referenced_ids: RefCell<HashSet<String>>,
    xmlns_stack: RefCell<NamespaceStack<'input>>,
}

pub(crate) struct RuleData<'e, 'input> {
    reports: cell::RefMut<'e, Vec<Error<'input>>>,
    parent: Option<ElementId<'input>>,
    element: &'e ElementId<'input>,
    attributes: cell::Ref<'e, [Attr<'input>]>,
    range: &'e Option<Range<usize>>,
    attribute_ranges: &'e HashMap<AttrId<'input>, Ranges>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for Rules {
    type Error = Vec<Error<'input>>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_script(document);
        context.query_has_stylesheet(document);

        let mut reporter = Reporter {
            rules: self,
            reports: RefCell::default(),
            ids: RefCell::default(),
            referenced_ids: RefCell::default(),
            xmlns_stack: RefCell::default(),
        };
        for style_sheet in &context.query_has_stylesheet_result {
            style_sheet.borrow_mut().visit(&mut reporter).ok();
        }
        reporter.start_with_context(document, context)?;
        let reports = reporter.reports.take();
        if reports.is_empty() {
            Ok(PrepareOutcome::skip)
        } else {
            Err(reports)
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for Reporter<'_, 'input> {
    type Error = Vec<Error<'input>>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut rule_data = RuleData::new(element, self.reports.borrow_mut());

        match &self.rules.no_unknown_elements {
            Severity::Off => {}
            severity => no_unknown_elements::no_unknown_elements(&mut rule_data, *severity),
        }
        match &self.rules.no_unknown_attributes {
            Severity::Off => {}
            severity => no_unknown_attributes::no_unknown_attributes(&mut rule_data, *severity),
        }
        match &self.rules.no_deprecated {
            Severity::Off => {}
            severity => no_deprecated::no_deprecated(&mut rule_data, *severity),
        }
        match &self.rules.no_default_attributes {
            Severity::Off => {}
            severity => no_default_attributes::no_default_attributes(&mut rule_data, *severity),
        }
        match &self.rules.no_x_link {
            Severity::Off => {}
            severity => no_xlink::no_xlink(&mut rule_data, *severity),
        }
        match &self.rules.no_invalid_attributes {
            Severity::Off => {}
            severity => no_invalid_attributes::no_invalid_attributes(&mut rule_data, *severity),
        }

        drop(rule_data.attributes);
        self.track_id_references(element);
        self.push_xmlns(element);
        self.track_xmlns(element);
        Ok(())
    }

    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut reports = self.reports.borrow_mut();
        if let Some(namespaces) = self.pop_xmlns() {
            match self.rules.no_unused_xmlns {
                Severity::Off => {}
                severity => {
                    no_unused_xmlns::no_unused_xmlns(
                        &mut reports,
                        element.range().as_ref(),
                        namespaces,
                        severity,
                    );
                }
            }
        }

        Ok(())
    }

    fn exit_document(
        &self,
        _document: &Element<'input, 'arena>,
        context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if context
            .flags
            .intersects(ContextFlags::query_has_script_result)
        {
            return Ok(());
        }

        let referenced_ids = self.referenced_ids.borrow();
        let ids = self.ids.borrow();
        let mut reports = self.reports.borrow_mut();

        match self.rules.no_unused_ids {
            Severity::Off => {}
            severity => no_unused_ids::no_unused_ids(&mut reports, &ids, &referenced_ids, severity),
        }
        Ok(())
    }
}
impl<'i> lightningcss::visitor::Visitor<'i> for Reporter<'_, '_> {
    type Error = ();

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        lightningcss::visit_types!(URLS)
    }
    fn visit_url(
        &mut self,
        url: &mut lightningcss::values::url::Url<'i>,
    ) -> Result<(), Self::Error> {
        if let Some(id) = url.url.strip_prefix('#') {
            self.referenced_ids.borrow_mut().insert(id.to_string());
        }
        Ok(())
    }
}
impl<'input> Reporter<'_, 'input> {
    fn track_id_references(&self, element: &Element<'input, '_>) {
        let mut referenced_ids = self.referenced_ids.borrow_mut();
        let attribute_ranges = element.attribute_ranges();
        for mut attribute in element.attributes().into_iter_mut() {
            if let Attr::Id(id) = attribute.unaliased() {
                self.ids
                    .borrow_mut()
                    .insert(id.0.clone(), attribute_ranges.get(&AttrId::Id).cloned());
                continue;
            }
            attribute.value_mut().visit_id(|id| {
                referenced_ids.insert(id.to_string());
            });
            attribute.value_mut().visit_url(|url| {
                if let Some(url) = url.strip_prefix('#') {
                    referenced_ids.insert(url.to_string());
                }
            });
        }
    }

    fn push_xmlns(&self, element: &Element<'input, '_>) {
        let mut namespaces = HashSet::new();
        for attr in element.attributes() {
            match &*attr {
                Attr::XMLNS(uri) => {
                    namespaces.insert((None, uri.clone(), false));
                }
                Attr::Unparsed { attr_id, value } => {
                    if let AttrId::Unknown(QualName {
                        prefix: Prefix::XMLNS,
                        local,
                    }) = &**attr_id
                    {
                        namespaces.insert((Some(local.clone()), value.clone(), false));
                    }
                }
                _ => {}
            }
        }
        self.xmlns_stack.borrow_mut().push(namespaces);
    }
    fn track_xmlns(&self, element: &Element<'input, '_>) {
        let mut xmlns_stack = self.xmlns_stack.borrow_mut();
        let insert = |xmlns_stack: &mut NamespaceStack<'input>,
                      name: Option<Atom<'input>>,
                      uri: &Atom<'input>| {
            let is_used_group = (name.clone(), uri.clone(), true);
            let not_used_group = (name, uri.clone(), false);

            for set in xmlns_stack.iter_mut().rev() {
                if set.contains(&is_used_group) {
                    break;
                } else if let Some(mut xmlns) = set.take(&not_used_group) {
                    xmlns.2 = true;
                    set.insert(xmlns);
                }
            }
        };
        for attr in element.attributes() {
            let prefix = attr.prefix();
            let name = prefix.value();
            let uri = prefix.ns().uri();
            insert(&mut xmlns_stack, name, uri);
        }
        let prefix = element.prefix();
        let name = prefix.value();
        let uri = prefix.ns().uri();
        insert(&mut xmlns_stack, name, uri);
    }
    fn pop_xmlns(&self) -> Option<HashSet<(Option<Atom<'input>>, Atom<'input>, bool)>> {
        self.xmlns_stack.borrow_mut().pop()
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => Ok(()),
            Self::Warn => f.write_char('⚠'),
            Self::Error => f.write_char('×'),
        }
    }
}
impl Severity {
    /// Returns off variant of self, useful for passing as a serde default
    pub const fn off() -> Self {
        Self::Off
    }

    pub(crate) fn color_start(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => Ok(()),
            Self::Warn => f.write_str("\x1b[33m"),  // Yellow
            Self::Error => f.write_str("\x1b[31m"), // Red
        }
    }
    pub(crate) fn color_reset(f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\x1b[0m")
    }
}

#[cfg(test)]
struct TestData<'input> {
    reports: RefCell<Vec<Error<'input>>>,
    parent: Option<ElementId<'input>>,
    element: ElementId<'input>,
    attributes: RefCell<Vec<Attr<'input>>>,
    range: Option<Range<usize>>,
    attribute_ranges: HashMap<AttrId<'input>, Ranges>,
}
impl<'e, 'input> RuleData<'e, 'input> {
    fn new(
        element: &'e Element<'input, '_>,
        reports: cell::RefMut<'e, Vec<Error<'input>>>,
    ) -> Self {
        Self {
            reports,
            parent: element
                .parent_element()
                .as_ref()
                .map(Element::qual_name)
                .cloned(),
            element: element.qual_name(),
            attributes: element.attributes().as_slice(),
            range: element.range(),
            attribute_ranges: element.attribute_ranges(),
        }
    }
    #[cfg(test)]
    fn test_data() -> TestData<'input> {
        TestData {
            reports: RefCell::new(vec![]),
            parent: Some(ElementId::Svg),
            element: ElementId::Svg,
            attributes: RefCell::new(vec![]),
            range: Some(0..1),
            attribute_ranges: HashMap::new(),
        }
    }
    #[cfg(test)]
    fn from_test_data(test_data: &'e TestData<'input>) -> Self {
        Self {
            reports: test_data.reports.borrow_mut(),
            parent: test_data.parent.clone(),
            element: &test_data.element,
            attributes: cell::Ref::map(test_data.attributes.borrow(), |a| a.as_slice()),
            range: &test_data.range,
            attribute_ranges: &test_data.attribute_ranges,
        }
    }
}
