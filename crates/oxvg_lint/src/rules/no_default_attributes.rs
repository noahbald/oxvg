use std::collections::HashMap;

use oxvg_ast::node::Ranges;
use oxvg_collections::attribute::{Attr, AttrId};
use rayon::prelude::*;

use crate::error::{Error, Problem};

use super::Severity;

pub fn no_default_attributes<'a, 'input>(
    attributes: &'a [Attr<'input>],
    attribute_ranges: &'a HashMap<AttrId<'input>, Ranges>,
    severity: Severity,
) -> impl ParallelIterator<Item = Error<'input>> + use<'a, 'input> {
    attributes.par_iter().filter_map(move |attr| {
        let name = attr.name().clone();
        if Some(attr) == name.default().as_ref() {
            Some(Error {
                range: attribute_ranges.get(&name).map(|range| range.value.clone()),
                problem: Problem::DefaultAttribute(name),
                severity,
                help: None,
            })
        } else {
            None
        }
    })
}

#[cfg(test)]
mod test {
    use super::no_default_attributes;
    use crate::{error::Problem, Severity};
    use oxvg_ast::node::Ranges;
    use oxvg_collections::attribute::{uncategorised::Target, Attr, AttrId};
    use rayon::iter::ParallelIterator as _;
    use std::collections::HashMap;

    static ATTR_NOT_DEFAULT: Attr = Attr::Target(Target::_Blank);
    static ATTR_DEFAULT: Attr = Attr::Target(Target::_Self);
    static ATTR_ID: AttrId = AttrId::Target;

    #[test]
    fn report_no_default_attributes_ok() {
        let report: Vec<_> = no_default_attributes(
            &[ATTR_NOT_DEFAULT.clone()],
            &HashMap::new(),
            Severity::Error,
        )
        .collect();
        assert!(report.is_empty());
    }

    #[test]
    fn report_no_default_attributes_error() {
        let report: Vec<_> = no_default_attributes(
            &[ATTR_DEFAULT.clone()],
            &HashMap::from([(
                ATTR_ID.clone(),
                Ranges {
                    range: 0..1,
                    name: 1..2,
                    value: 2..3,
                },
            )]),
            Severity::Error,
        )
        .collect();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::DefaultAttribute(ATTR_ID.clone())
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(2..3));
        assert_eq!(report[0].help, None);
    }
}
