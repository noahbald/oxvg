use std::{
    io::{StdoutLock, Write},
    path::PathBuf,
};

use oxvg_ast::{
    parse::roxmltree::{self, ParsingOptions},
    visitor::Visitor,
};

use crate::{
    error::{LintingError, Report},
    Rules, Severity,
};

struct StdoutWriter {
    lock: StdoutLock<'static>,
}
impl std::fmt::Write for StdoutWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.lock
            .write_all(s.as_bytes())
            .map_err(|_| std::fmt::Error)
    }
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.lock.write_fmt(args).map_err(|_| std::fmt::Error)
    }
}

impl Rules {
    /// Returns a set of rules with all the rules set to [`Severity::Off`]
    pub fn off() -> Self {
        Self {
            no_unknown_elements: Severity::Off,
            no_unknown_attributes: Severity::Off,
            no_deprecated: Severity::Off,
            no_default_attributes: Severity::Off,
            no_x_link: Severity::Off,
        }
    }

    /// Returns a balanced set of rules
    pub fn recommended() -> Self {
        Self {
            no_unknown_elements: Severity::Error,
            no_unknown_attributes: Severity::Error,
            no_deprecated: Severity::Error,
            no_default_attributes: Severity::Warn,
            no_x_link: Severity::Warn,
        }
    }

    /// Analyses the file and reports any problems to standard output
    ///
    /// # Errors
    ///
    /// When parsing fails or writing to standard output fails
    pub fn lint(&self, source: &str) -> Result<(), LintingError> {
        let lock = std::io::stdout().lock();
        let mut stdout = StdoutWriter { lock };
        self.lint_to(&mut stdout, source)
    }

    /// Analyses the file and reports any problems to the given writer
    ///
    /// # Errors
    ///
    /// When parsing fails or writing fails
    pub fn lint_to<W>(&self, w: &mut W, source: &str) -> Result<(), LintingError>
    where
        W: std::fmt::Write,
    {
        self.lint_internal(w, None, source)
    }

    /// Analyses the file and reports any problems to the given writer
    ///
    /// # Errors
    ///
    /// When parsing fails or writing fails
    pub fn lint_to_with_path<W>(
        &self,
        w: &mut W,
        source: &str,
        path: Option<&PathBuf>,
    ) -> Result<(), LintingError>
    where
        W: std::fmt::Write,
    {
        self.lint_internal(w, path, source)
    }

    /// Loads and analyses the file, reporting any problems to standard output
    ///
    /// # Errors
    ///
    /// When reading, parsing, or writing fails
    pub fn lint_from(&self, path: &PathBuf) -> Result<(), LintingError> {
        let file = std::fs::read_to_string(path).map_err(LintingError::IO)?;
        let lock = std::io::stdout().lock();
        let mut stdout = StdoutWriter { lock };
        self.lint_internal(&mut stdout, Some(path), &file)
    }

    /// Loads and analyses the file, reporting any problems to standard output
    ///
    /// # Errors
    ///
    /// When parsing, or writing fails
    pub fn lint_with_path(&self, source: &str, path: Option<&PathBuf>) -> Result<(), LintingError> {
        let lock = std::io::stdout().lock();
        let mut stdout = StdoutWriter { lock };
        self.lint_internal(&mut stdout, path, source)
    }

    /// Loads and analyses the file, reporting any problems to the given writer
    ///
    /// # Errors
    ///
    /// When reading, parsing, or writing fails
    pub fn lint_from_to<W>(&self, w: &mut W, path: &PathBuf) -> Result<(), LintingError>
    where
        W: std::fmt::Write,
    {
        let file = std::fs::read_to_string(path).map_err(LintingError::IO)?;
        self.lint_internal(w, Some(path), &file)
    }

    pub(crate) fn lint_internal<W>(
        &self,
        mut w: &mut W,
        path: Option<&PathBuf>,
        source: &str,
    ) -> Result<(), LintingError>
    where
        W: std::fmt::Write,
    {
        let mut error_count = 0;
        let mut warning_count = 0;
        let result = roxmltree::parse_with_options(
            source,
            ParsingOptions {
                allow_dtd: true,
                ..ParsingOptions::default()
            },
            |root, allocator| {
                let Err(errors) = self.start_with_path(root, allocator, path.cloned()) else {
                    return Ok(());
                };
                error_count += errors
                    .iter()
                    .filter(|error| matches!(error.severity, Severity::Error))
                    .count();
                warning_count += errors
                    .iter()
                    .filter(|error| matches!(error.severity, Severity::Warn))
                    .count();

                let report = Report {
                    source,
                    errors,
                    path: path.cloned(),
                };
                write!(&mut w, "{report}")
            },
        );
        match result {
            Err(err) => return Err(LintingError::Parse(err)),
            Ok(Err(err)) => return Err(LintingError::Format(err)),
            Ok(Ok(())) => {}
        }
        if error_count > 0 || warning_count > 0 {
            Err(LintingError::Reported {
                errors: error_count,
                warnings: warning_count,
            })
        } else {
            Ok(())
        }
    }
}
