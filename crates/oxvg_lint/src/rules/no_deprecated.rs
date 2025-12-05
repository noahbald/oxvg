use oxvg_collections::{attribute::AttributeInfo, element::ElementInfo};
use rayon::prelude::*;

use crate::error::{DeprecatedProblem, Error, Problem};

use super::{RuleData, Severity};

pub fn no_deprecated<'input>(
    RuleData {
        reports,
        element,
        attributes,
        range,
        attribute_ranges,
        ..
    }: &mut RuleData<'_, 'input>,
    severity: Severity,
) {
    if element.info().intersects(ElementInfo::Legacy) {
        reports.push(Error {
            problem: Problem::Deprecated(DeprecatedProblem::DeprecatedElement(element.clone())),
            severity,
            range: range.as_ref().map(|range| range.start..range.start),
            help: None,
        })
    };

    reports.par_extend(attributes.par_iter().filter_map(move |attr| {
        let attr_id = attr.name();
        if !attr_id
            .info()
            .intersects(AttributeInfo::DeprecatedSafe.union(AttributeInfo::DeprecatedUnsafe))
        {
            return None;
        }
        Some(Error {
            problem: Problem::Deprecated(DeprecatedProblem::DeprecatedAttribute(attr_id.clone())),
            severity,
            range: attribute_ranges
                .get(attr_id)
                .map(|range| range.name.clone()),
            help: if attr_id.info().intersects(AttributeInfo::DeprecatedSafe) {
                Some(String::from("This attribute can be safely removed"))
            } else {
                None
            },
        })
    }))
}

#[cfg(test)]
mod test {
    use super::no_deprecated;
    use crate::{
        error::{DeprecatedProblem, Problem},
        rules::RuleData,
        Severity,
    };
    use oxvg_ast::node::Ranges;
    use oxvg_collections::{
        atom::Atom,
        attribute::{uncategorised::ViewBox, xml::XmlSpace, Attr, AttrId},
        element::ElementId,
    };

    const OK_ELEMENT: ElementId = ElementId::Svg;
    const LEGACY_ELEMENT: ElementId = ElementId::TRef;
    const OK_ATTRIBUTE: Attr = Attr::ViewBox(ViewBox {
        min_x: 0.0,
        min_y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    const DEPRECATED_SAFE_ATTRIBUTE: Attr = Attr::Version(Atom::Static("1.1"));
    const DEPRECATED_UNSAFE_ATTRIBUTE: Attr = Attr::XmlSpace(XmlSpace::Default);
    const DEPRECATED_SAFE_ATTR_ID: AttrId = AttrId::Version;
    const DEPRECATED_UNSAFE_ATTR_ID: AttrId = AttrId::XmlSpace;

    #[test]
    fn report_deprecated_ok() {
        let mut test_data = RuleData::test_data();
        test_data.element = OK_ELEMENT;
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[OK_ATTRIBUTE]);
        let mut test_data = RuleData::from_test_data(&test_data);
        no_deprecated(&mut test_data, Severity::Error);
        assert!(test_data.reports.is_empty());
    }
    #[test]
    fn report_deprecated_element() {
        let mut test_data = RuleData::test_data();
        test_data.element = LEGACY_ELEMENT;
        let mut test_data = RuleData::from_test_data(&test_data);
        no_deprecated(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::Deprecated(DeprecatedProblem::DeprecatedElement(LEGACY_ELEMENT))
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..0));
        assert_eq!(report[0].help, None);
    }
    #[test]
    fn report_deprecated_attribute_safe() {
        let mut test_data = RuleData::test_data();
        test_data.element = OK_ELEMENT;
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[DEPRECATED_SAFE_ATTRIBUTE]);
        test_data.attribute_ranges.insert(
            DEPRECATED_SAFE_ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);
        no_deprecated(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::Deprecated(DeprecatedProblem::DeprecatedAttribute(
                DEPRECATED_SAFE_ATTR_ID
            ))
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(1..2));
        assert_eq!(
            report[0].help,
            Some(String::from("This attribute can be safely removed"))
        );
    }
    #[test]
    fn report_deprecated_attribute_unsafe() {
        let mut test_data = RuleData::test_data();
        test_data.element = OK_ELEMENT;
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[DEPRECATED_UNSAFE_ATTRIBUTE]);
        test_data.attribute_ranges.insert(
            DEPRECATED_UNSAFE_ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);
        no_deprecated(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::Deprecated(DeprecatedProblem::DeprecatedAttribute(
                DEPRECATED_UNSAFE_ATTR_ID.clone()
            ))
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(1..2));
        assert_eq!(report[0].help, None);
    }
}
