use rayon::prelude::*;

use crate::error::{Error, Problem};

use super::{RuleData, Severity};

pub fn no_default_attributes<'a, 'input>(
    RuleData {
        reports,
        attributes,
        attribute_ranges,
        ..
    }: &mut RuleData<'_, 'input>,
    severity: Severity,
) {
    reports.par_extend(attributes.par_iter().filter_map(move |attr| {
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
    }))
}

#[cfg(test)]
mod test {
    use super::no_default_attributes;
    use crate::{error::Problem, rules::RuleData, Severity};
    use oxvg_ast::node::Ranges;
    use oxvg_collections::attribute::{uncategorised::Target, Attr, AttrId};

    const ATTR_NOT_DEFAULT: Attr = Attr::Target(Target::_Blank);
    const ATTR_DEFAULT: Attr = Attr::Target(Target::_Self);
    const ATTR_ID: AttrId = AttrId::Target;

    #[test]
    fn report_no_default_attributes_ok() {
        let test_data = RuleData::test_data();
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[ATTR_NOT_DEFAULT]);
        let mut test_data = RuleData::from_test_data(&test_data);
        no_default_attributes(&mut test_data, Severity::Error);
        assert!(test_data.reports.is_empty());
    }

    #[test]
    fn report_no_default_attributes_error() {
        let mut test_data = RuleData::test_data();
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[ATTR_DEFAULT]);
        test_data.attribute_ranges.insert(
            ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);
        no_default_attributes(&mut test_data, Severity::Error);
        let report = test_data.reports;
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
