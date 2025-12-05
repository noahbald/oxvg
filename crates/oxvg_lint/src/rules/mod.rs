use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Write,
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
};
use rayon::prelude::*;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::Error;

mod no_default_attributes;
mod no_deprecated;
mod no_unknown_attributes;
mod no_unknown_elements;
mod no_unused_ids;
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
}

struct Reporter<'o, 'input> {
    rules: &'o Rules,
    reports: RefCell<Vec<Error<'input>>>,
    ids: RefCell<HashMap<Atom<'input>, Option<Ranges>>>,
    referenced_ids: RefCell<HashSet<String>>,
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
        let parent = element.parent_element();
        let parent = parent.as_ref();
        let parent_name = parent.map(Element::qual_name);
        let name = element.qual_name();
        let range = element.range();
        let attributes = element.attributes();
        let attributes_slice = attributes.as_slice();
        let attribute_ranges = element.attribute_ranges();
        let mut reports = self.reports.borrow_mut();

        match &self.rules.no_unknown_elements {
            Severity::Off => {}
            severity => {
                if let Some(r) = no_unknown_elements::no_unknown_elements(
                    parent_name,
                    name,
                    range.as_ref(),
                    *severity,
                ) {
                    reports.push(r);
                };
            }
        }
        match &self.rules.no_unknown_attributes {
            Severity::Off => {}
            severity => {
                if let Some(r) = no_unknown_attributes::no_unknown_attributes(
                    name,
                    &attributes_slice,
                    attribute_ranges,
                    *severity,
                ) {
                    reports.par_extend(r);
                }
            }
        }
        match &self.rules.no_deprecated {
            Severity::Off => {}
            severity => {
                reports.par_extend(no_deprecated::no_deprecated(
                    name,
                    &attributes_slice,
                    range.as_ref(),
                    attribute_ranges,
                    *severity,
                ));
            }
        }
        match &self.rules.no_default_attributes {
            Severity::Off => {}
            severity => {
                reports.par_extend(no_default_attributes::no_default_attributes(
                    &attributes_slice,
                    attribute_ranges,
                    *severity,
                ));
            }
        }
        match &self.rules.no_x_link {
            Severity::Off => {}
            severity => reports.par_extend(no_xlink::no_xlink(
                &attributes_slice,
                attribute_ranges,
                *severity,
            )),
        }

        drop(attributes_slice);
        let mut referenced_ids = self.referenced_ids.borrow_mut();
        for mut attribute in attributes.into_iter_mut() {
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
            severity => reports.par_extend(no_unused_ids::no_unused_ids(
                &ids,
                &referenced_ids,
                severity,
            )),
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
