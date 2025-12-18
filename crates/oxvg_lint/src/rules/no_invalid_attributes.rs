use oxvg_collections::attribute::{
    path::{Path, Points},
    Attr, AttrId,
};
use rayon::prelude::*;

use crate::error::{Error, Problem};

use super::{RuleData, Severity};

pub fn no_invalid_attributes(
    RuleData {
        reports,
        attributes,
        attribute_ranges,
        ..
    }: &mut RuleData,
    severity: Severity,
) {
    reports.par_extend(attributes.par_iter().filter_map(|attr| {
        let attr_id = attr.name();
        match attr {
            Attr::Unparsed { attr_id, value } if !matches!(&**attr_id, AttrId::Unknown(_)) => {
                Some(Error {
                    problem: Problem::InvalidAttribute {
                        attribute: *attr_id.clone(),
                    },
                    severity,
                    range: attribute_ranges.get(attr_id).map(|r| r.value.clone()),
                    help: Some(format!(
                        r#"The value "{value}" could not be parsed as `{:?}`"#,
                        attr_id.r#type()
                    )),
                })
            }
            Attr::Path(Path(_, Some(unparsed)))
            | Attr::D(Path(_, Some(unparsed)))
            | Attr::Points(Points(_, Some(unparsed))) => Some(Error {
                problem: Problem::InvalidAttribute {
                    attribute: attr_id.clone(),
                },
                severity,
                range: attribute_ranges
                    .get(attr_id)
                    .map(|r| (r.value.end - unparsed.len())..r.value.end),
                help: Some("The path could not be fully parsed".to_string()),
            }),
            _ => None,
        }
    }));
}

#[cfg(test)]
mod test {
    use super::no_invalid_attributes;
    use crate::{error::Problem, rules::RuleData, Severity};
    use oxvg_ast::node::Ranges;
    use oxvg_collections::{
        atom::Atom,
        attribute::{path::Path, Attr, AttrId},
    };

    const ATTR_ID: AttrId<'static> = AttrId::Path;
    const VALID_ATTR: Attr<'static> = Attr::Path(Path(oxvg_path::Path(vec![]), None));
    const PARTIAL_VALID_ATTR: Attr<'static> = Attr::Path(Path(oxvg_path::Path(vec![]), Some("M0")));

    #[test]
    fn report_invalid_attributes_ok() {
        let test_data = RuleData::test_data();
        test_data.attributes.borrow_mut().push(VALID_ATTR);
        let mut test_data = RuleData::from_test_data(&test_data);

        no_invalid_attributes(&mut test_data, Severity::Error);
        assert!(test_data.reports.is_empty());
    }

    #[test]
    fn report_invalid_attributes_partial_path() {
        let mut test_data = RuleData::test_data();
        test_data.attributes.borrow_mut().push(PARTIAL_VALID_ATTR);
        test_data.attribute_ranges.insert(
            ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 0..10,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);

        no_invalid_attributes(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::InvalidAttribute { attribute: ATTR_ID }
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(8..10));
        assert_eq!(
            report[0].help,
            Some("The path could not be fully parsed".to_string())
        );
    }

    #[test]
    fn report_invalid_attributes_err() {
        let mut test_data = RuleData::test_data();
        test_data.attributes.borrow_mut().push(Attr::Unparsed {
            attr_id: Box::new(AttrId::Path),
            value: Atom::Static("foo"),
        });
        test_data.attribute_ranges.insert(
            ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);

        no_invalid_attributes(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::InvalidAttribute { attribute: ATTR_ID }
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(2..3));
        assert_eq!(
            report[0].help,
            Some(r#"The value "foo" could not be parsed as `Path`"#.to_string(),)
        );
    }
}
