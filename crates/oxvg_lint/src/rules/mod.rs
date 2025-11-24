use std::{cell::RefCell, fmt::Write};

use oxvg_ast::{
    element::Element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use rayon::prelude::*;

use crate::error::Error;

mod no_unknown_attributes;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
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

#[derive(Default)]
/// A set of rules to assert against a document.
///
/// The [`Severity`] provided for each rule determines the display of each attribute
/// by the [`crate::error::Report`].
pub struct Rules {
    /// Disallow using attribute that do not belong to a known element's content model
    pub no_unknown_attributes: Severity,
}

struct Reporter<'o, 'input> {
    rules: &'o Rules,
    reports: RefCell<Vec<Error<'input>>>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for Rules {
    type Error = Vec<Error<'input>>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        let reporter = Reporter {
            rules: self,
            reports: RefCell::default(),
        };
        reporter.start(&mut document.clone(), context.info, None)?;
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
        let name = element.qual_name();
        let attributes = element.attributes();
        let attributes_slice = attributes.as_slice();
        let attribute_ranges = element.attribute_ranges();

        let no_unknown_attributes = match &self.rules.no_unknown_attributes {
            Severity::Off => None,
            severity => no_unknown_attributes::no_unknown_attributes(
                name,
                &attributes_slice,
                attribute_ranges,
                *severity,
            ),
        };

        let mut reports = self.reports.borrow_mut();
        if let Some(r) = no_unknown_attributes {
            reports.par_extend(r);
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
