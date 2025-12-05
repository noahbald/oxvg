use std::collections::{HashMap, HashSet};

use oxvg_ast::node::Ranges;
use oxvg_collections::atom::Atom;
use rayon::prelude::*;

use crate::error::{Error, Problem};

use super::Severity;

pub fn no_unused_ids<'a, 'input>(
    ids: &'a HashMap<Atom<'input>, Option<Ranges>>,
    referenced_ids: &'a HashSet<String>,
    severity: Severity,
) -> impl ParallelIterator<Item = Error<'input>> + use<'a, 'input> {
    ids.par_iter().filter_map(move |(id, ranges)| {
        if referenced_ids.contains(id.as_str()) {
            None
        } else {
            Some(Error {
                problem: Problem::UnreferencedId(id.clone()),
                severity,
                range: ranges.as_ref().map(|ranges| ranges.value.clone()),
                help: None,
            })
        }
    })
}

#[cfg(test)]
mod test {
    use super::no_unused_ids;
    use crate::{error::Problem, Severity};
    use oxvg_ast::node::Ranges;
    use oxvg_collections::atom::Atom;
    use rayon::iter::ParallelIterator as _;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn report_no_unused_ids_ok() {
        let ids = HashMap::from([(
            Atom::Static("foo"),
            Some(Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            }),
        )]);
        let referenced_ids = HashSet::from([String::from("foo")]);
        let report: Vec<_> = no_unused_ids(&ids, &referenced_ids, Severity::Error).collect();
        assert!(report.is_empty());
    }

    #[test]
    fn report_no_unused_ids_error() {
        let ids = HashMap::from([(
            Atom::Static("foo"),
            Some(Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            }),
        )]);
        let referenced_ids = HashSet::from([]);
        let report: Vec<_> = no_unused_ids(&ids, &referenced_ids, Severity::Error).collect();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::UnreferencedId(Atom::Static("foo"))
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(2..3));
        assert_eq!(report[0].help, None);
    }
}
