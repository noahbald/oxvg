//! Errors that may be created or reported by the linter
use core::str;
use std::{
    fmt::{self, Display, Write},
    ops::Range,
    path::PathBuf,
};

use oxvg_collections::{attribute::AttrId, element::ElementId};

use crate::{utils::naive_range, Severity};

#[derive(Debug)]
/// Errors that may occur while linting
pub enum LintingError {
    #[cfg(feature = "parse")]
    /// The linter was unable to read the input
    IO(std::io::Error),
    #[cfg(feature = "parse")]
    /// The linter was unable to parse the document
    Parse(oxvg_ast::parse::roxmltree::ParseError),
    /// The linter was unable to format the report
    Format(std::fmt::Error),
    /// The linter reported some errors and/or warnings
    Reported {
        /// The number of errors reported
        errors: usize,
        /// The number of warnings reported
        warnings: usize,
    },
}

#[derive(Debug, PartialEq)]
/// A problem with the document that the linter reported
pub enum Problem<'input> {
    /// There was an attribute unknown for the given element
    UnknownAttribute {
        /// The unknown attribute
        attribute: AttrId<'input>,
        /// The element the attribute was found on
        element: ElementId<'input>,
    },
    /// There was an SVG element unknown for its parent's content-model
    UnknownElement {
        /// The element that contains the unknown element
        parent: ElementId<'input>,
        /// The element that's unknown for its parent's content-model
        element: ElementId<'input>,
    },
    /// There was an attribute or element that's marked as deprecated and may be removed in the future
    Deprecated(DeprecatedProblem<'input>),
    /// There was an attribute with a value that matches its default
    DefaultAttribute(AttrId<'input>),
    /// There was an `xlink`-prefixed attribute used in the document.
    NoXLink(NoXLinkProblem<'input>),
}
impl Display for Problem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAttribute { attribute, element } => f.write_fmt(format_args!(
                "Unknown attribute `{attribute}` for element <{element}>"
            )),
            Self::UnknownElement { parent, element } => f.write_fmt(format_args!(
                "Unknown element <{element}> for parent <{parent}>"
            )),
            Self::Deprecated(problem) => problem.fmt(f),
            Self::DefaultAttribute(attribute) => f.write_fmt(format_args!("The attribute `{attribute}` has a value that matches its default and can be safely omitted")),
            Self::NoXLink(problem) => problem.fmt(f)
        }
    }
}

#[derive(Debug, PartialEq)]
/// Either an element or an attribute is deprecated
pub enum DeprecatedProblem<'input> {
    /// The specified element is deprecated
    DeprecatedElement(ElementId<'input>),
    /// The specified attribute is deprecated
    DeprecatedAttribute(AttrId<'input>),
}
impl Display for DeprecatedProblem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let support = "will be removed in SVG 2 and may not be supported by modern clients.";
        match self {
            Self::DeprecatedElement(element) => {
                f.write_fmt(format_args!("Deprecated element <{element}> {support}"))
            }
            Self::DeprecatedAttribute(attribute) => {
                f.write_fmt(format_args!("Deprecated attribute `{attribute}` {support}"))
            }
        }
    }
}

#[derive(Debug, PartialEq)]
/// Different variants of an `xlink` being used
pub enum NoXLinkProblem<'input> {
    /// `xlink:show="replace"` was used.
    XLinkShowReplace,
    /// `xlink:show="new"` was used.
    XLinkShowNew,
    /// `xlink:title` was used.
    XLinkTitle,
    /// `xlink:href` was used.
    XLinkHref,
    /// An `xlink`-prefixed attribute was used.
    XLinkUnsupported(AttrId<'input>),
}
static XLINK_SHOW_PRELUDE: &str =
    "The attribute `xlink:show` is deprecated and may be replaced with";
impl Display for NoXLinkProblem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XLinkShowReplace => {
                f.write_fmt(format_args!(r#"{XLINK_SHOW_PRELUDE} `target="_new"`."#))
            }
            Self::XLinkShowNew => {
                f.write_fmt(format_args!(r#"{XLINK_SHOW_PRELUDE} `target="_blank"`."#))
            }
            Self::XLinkTitle => f.write_str(
                "The attribute `xlink:title` is deprecated and may be replaced with `<title>`.",
            ),
            Self::XLinkHref => f.write_str(
                "The attribute `xlink:href` is deprecated and may be replaced with `href`.",
            ),
            Self::XLinkUnsupported(attr) => f.write_fmt(format_args!(
                "The attribute `{attr}` is deprecated and may not be supported by clients."
            )),
        }
    }
}

#[derive(Debug)]
/// An error for a problem the linter reported for the document
pub struct Error<'input> {
    /// The problem that was found
    pub problem: Problem<'input>,
    /// The error level of the problem
    pub severity: Severity,
    /// The span of the document the error was reported for
    pub range: Option<Range<usize>>,
    /// Some arbitrary help text provided by the reporter
    pub help: Option<String>,
}
impl Error<'_> {
    fn display_context(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        source: &str,
        path: Option<&PathBuf>,
    ) -> std::fmt::Result {
        let path = path.and_then(|path| path.to_str()).unwrap_or("");
        let Some(mut range) = self.range.clone() else {
            return f.write_fmt(format_args!(" \x1b[1;34m{path}\x1b"));
        };
        let source_bytes = source.as_bytes();
        if range.start == range.end {
            range = naive_range(source_bytes, range.start);
        }
        let line_start = source_bytes[..range.start]
            .iter()
            .rposition(|char| *char == b'\n')
            .map_or(0, |i| i + 1);
        let line_end = source_bytes[line_start..]
            .iter()
            .position(|char| *char == b'\n')
            .map_or(source_bytes.len(), |i| i + line_start);

        let prev_line = str::from_utf8(
            source_bytes[..line_start.saturating_sub(1)]
                .rsplit(|char| *char == b'\n')
                .next()
                .unwrap(),
        )
        .map_err(|_| std::fmt::Error)?;
        let next_line = str::from_utf8(
            source_bytes[(source_bytes.len()).min(line_end + 1)..]
                .split(|char| *char == b'\n')
                .next()
                .unwrap(),
        )
        .map_err(|_| std::fmt::Error)?;
        let lines =
            std::str::from_utf8(&source_bytes[line_start..line_end]).map_err(|_| fmt::Error)?;

        let line_number = bytecount::count(&source_bytes[..range.start], b'\n') + 1;
        let column = range.start - line_start;
        let padding = (line_number + lines.split('\n').count()).to_string().len();
        f.write_fmt(format_args!(
            "\n {: <1$} ╭─[\x1b[34m{path}\x1b[0m:{line_number}:{column}]",
            "", padding,
        ))?;

        let mut current_line_number = line_number - 1;
        if !prev_line.is_empty() {
            f.write_fmt(format_args!(
                "\n {current_line_number: >padding$} │ {prev_line}"
            ))?;
        }

        for line in lines.split('\n') {
            current_line_number += 1;
            f.write_fmt(format_args!("\n {current_line_number: >padding$} │ {line}"))?;
        }
        if current_line_number > line_number + 1 {
            f.write_fmt(format_args!("\n {: <padding$} · \x1b[35m", ""))?;
            f.write_fmt(format_args!("{:─<1$}\x1b[0m", "", column + range.len()))?;
        } else {
            f.write_fmt(format_args!("\n {: <1$} · ", "", padding,))?;
            f.write_fmt(format_args!("{: <1$}\x1b[35m", "", column))?;
            f.write_fmt(format_args!("{:─<1$}\x1b[0m", "", range.len()))?;
        }

        if !next_line.is_empty() {
            current_line_number += 1;
            f.write_fmt(format_args!("\n {current_line_number} │ {next_line}"))?;
        }

        f.write_fmt(format_args!("\n {: <1$} ╰────\n", "", padding))?;
        if let Some(help) = &self.help {
            f.write_fmt(format_args!(" \x1b[36mhelp:\x1b[0m {help}\n"))?;
        }
        Ok(())
    }
}
impl Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(' ')?;
        self.severity.color_start(f)?;
        self.severity.fmt(f)?;
        f.write_char(' ')?;
        self.problem.fmt(f)?;
        Severity::color_reset(f)
    }
}

#[derive(Debug)]
/// The list of errors reported for the given document
pub struct Report<'input> {
    /// The source data that the errors were reported for
    pub source: &'input str,
    /// The list of errors that the linter found for the document
    pub errors: Vec<Error<'input>>,
    /// The path of the source data
    pub path: Option<PathBuf>,
}
impl std::fmt::Display for Report<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in &self.errors {
            error.fmt(f)?;
            error.display_context(f, self.source, self.path.as_ref())?;
        }
        Ok(())
    }
}
impl std::error::Error for Report<'_> {}

impl std::fmt::Display for LintingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO(err) => err.fmt(f),
            Self::Parse(err) => err.fmt(f),
            Self::Format(err) => err.fmt(f),
            Self::Reported { errors, warnings } => f.write_fmt(format_args!(
                "Found {warnings} warning{} and {errors} error{}.",
                if *warnings == 1 { "" } else { "s" },
                if *errors == 1 { "" } else { "s" }
            )),
        }
    }
}
impl std::error::Error for LintingError {}

#[test]
fn print_single_error_single_line() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Error,
            ..crate::Rules::off()
        }
        .lint_to(&mut result, r#"<svg foo="bar" />"#)
        .ok();

        insta::assert_snapshot!(result);
    }
}
#[test]
fn print_many_error_single_line() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Error,
            ..crate::Rules::off()
        }
        .lint_to(&mut result, r#"<svg foo="bar" bar="baz" />"#)
        .ok();

        insta::assert_snapshot!(result);
    }
}
#[test]
fn print_single_error_many_line() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Error,
            ..crate::Rules::off()
        }
        .lint_to(
            &mut result,
            r#"<svg>
    <rect x="10" y="10" width="10" height="10" foo="bar" />
</svg>"#,
        )
        .ok();

        insta::assert_snapshot!(result);
    }
}
#[test]
fn print_many_error_many_line() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Error,
            ..crate::Rules::off()
        }
        .lint_to(
            &mut result,
            r#"<svg foo="bar">
    <rect x="10" y="10" width="10" height="10" bar="baz" />
</svg>"#,
        )
        .ok();

        insta::assert_snapshot!(result);
    }
}
#[test]
fn print_warning() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Warn,
            ..crate::Rules::off()
        }
        .lint_to(&mut result, r#"<svg foo="bar" />"#)
        .ok();

        insta::assert_snapshot!(result);
    }
}
#[test]
fn print_help() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Error,
            ..crate::Rules::off()
        }
        .lint_to(
            &mut result,
            r#"<svg xmlns:xlink="http://unknown.com">
    <a xlink:href="/foo">foo</a>
</svg>"#,
        )
        .ok();

        insta::assert_snapshot!(result);
    }
}
#[test]
fn print_path() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Error,
            ..crate::Rules::off()
        }
        .lint_to_with_path(
            &mut result,
            r#"<svg foo="bar" />"#,
            Some(&PathBuf::from("./file")),
        )
        .ok();

        insta::assert_snapshot!(result);
    }
}
#[test]
fn empty() {
    {
        let mut result = String::new();
        crate::Rules {
            no_unknown_attributes: Severity::Off,
            ..crate::Rules::off()
        }
        .lint_to(&mut result, r#"<svg foo="bar" />"#)
        .unwrap();
    }
}
