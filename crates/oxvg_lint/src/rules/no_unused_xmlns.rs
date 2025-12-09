use std::{collections::HashSet, ops::Range};

use oxvg_collections::atom::Atom;
use rayon::prelude::*;

use crate::error::{Error, Problem};

use super::Severity;

pub fn no_unused_xmlns<'input>(
    reports: &mut Vec<Error<'input>>,
    range: Option<&Range<usize>>,
    mut xmlns_set: HashSet<(Option<Atom<'input>>, Atom<'input>, bool)>,
    severity: Severity,
) {
    let range = range.map(|r| r.start..r.start);
    reports.par_extend(
        xmlns_set
            .par_drain()
            .filter_map(move |(prefix, uri, is_used)| {
                if is_used {
                    None
                } else {
                    Some(Error {
                        problem: Problem::UnreferencedXMLNS(prefix, uri),
                        severity,
                        range: range.clone(),
                        help: None,
                    })
                }
            }),
    );
}

#[cfg(test)]
mod test {
    use super::no_unused_xmlns;
    use crate::{error::Problem, Severity};
    use oxvg_collections::atom::Atom;
    use std::collections::HashSet;

    const PREFIX: Option<Atom> = Some(Atom::Static("foo"));
    const LOCAL: Atom = Atom::Static("bar");

    #[test]
    fn report_no_unused_xmlns_ok() {
        let mut reports = Vec::new();
        let xmlns_set = HashSet::from([(PREFIX, LOCAL, true)]);

        no_unused_xmlns(
            &mut reports,
            Some(0..1).as_ref(),
            xmlns_set,
            Severity::Error,
        );

        assert!(reports.is_empty());
    }

    #[test]
    fn report_no_unused_xmlns_error() {
        let mut report = Vec::new();
        let xmlns_set = HashSet::from([(PREFIX, LOCAL, false)]);

        no_unused_xmlns(&mut report, Some(0..1).as_ref(), xmlns_set, Severity::Error);

        assert_eq!(report.len(), 1);
        assert_eq!(report[0].problem, Problem::UnreferencedXMLNS(PREFIX, LOCAL));
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..0));
        assert_eq!(report[0].help, None);
    }
}
