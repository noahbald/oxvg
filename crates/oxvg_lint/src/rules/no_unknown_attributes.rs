use oxvg_collections::{element::ElementId, name::Prefix};
use rayon::prelude::*;

use crate::{
    error::{Error, Problem},
    utils::prefix_help,
};

use super::{RuleData, Severity};

pub fn no_unknown_attributes<'input>(
    RuleData {
        reports,
        element,
        attributes,
        attribute_ranges,
        ..
    }: &mut RuleData<'_, 'input>,
    severity: Severity,
) {
    if matches!(element, ElementId::Unknown(_)) {
        return;
    }

    let expected_attributes = element.expected_attributes();
    let expected_attribute_groups = element.expected_attribute_groups();
    reports.par_extend(attributes.par_iter().filter_map(move |attr| {
        let attr_id = attr.name();
        if expected_attributes.contains(attr_id)
            || expected_attribute_groups.intersects(attr_id.attribute_group())
            || *attr.prefix() == Prefix::XMLNS
        {
            None
        } else {
            let help = prefix_help(attr.prefix());
            Some(Error {
                problem: Problem::UnknownAttribute {
                    attribute: attr_id.clone(),
                    element: element.clone(),
                },
                severity,
                range: attribute_ranges
                    .get(attr_id)
                    .map(|range| range.range.clone()),
                help,
            })
        }
    }))
}

#[cfg(test)]
mod test {
    use super::no_unknown_attributes;
    use crate::{error::Problem, rules::RuleData, Severity};
    use oxvg_ast::node::Ranges;
    use oxvg_collections::{
        atom::Atom,
        attribute::{Attr, AttrId},
        element::ElementId,
        name::{Prefix, QualName, NS},
    };

    const UNKNOWN_ELEMENT: ElementId<'static> = ElementId::Unknown(QualName {
        prefix: Prefix::SVG,
        local: Atom::Static("foo"),
    });
    const KNOWN_ELEMENT: ElementId<'static> = ElementId::Svg;
    const UNKNOWN_ATTR_ID: AttrId<'static> = AttrId::Unknown(QualName {
        prefix: Prefix::SVG,
        local: Atom::Static("foo"),
    });
    const UNKNOWN_ATTR: Attr<'static> = Attr::Unparsed {
        attr_id: AttrId::Unknown(QualName {
            prefix: Prefix::SVG,
            local: Atom::Static("foo"),
        }),
        value: Atom::Static("bar"),
    };
    const UNKNOWN_PREFIXED_ATTR_ID: AttrId<'static> = AttrId::Unknown(QualName {
        prefix: Prefix::Unknown {
            prefix: Some(Atom::Static("xml")),
            ns: NS::Unknown(Atom::Static("https://unknown.org")),
        },
        local: Atom::Static("foo"),
    });
    const UNKNOWN_PREFIXED_ATTR: Attr<'static> = Attr::Unparsed {
        attr_id: AttrId::Unknown(QualName {
            prefix: Prefix::Unknown {
                prefix: Some(Atom::Static("xml")),
                ns: NS::Unknown(Atom::Static("https://unknown.org")),
            },
            local: Atom::Static("foo"),
        }),
        value: Atom::Static("bar"),
    };
    const KNOWN_ATTR: Attr<'static> = Attr::Version(Atom::Static("1.1"));

    #[test]
    fn report_unknown_attribute_ok() {
        let test_data = RuleData::test_data();
        test_data.attributes.borrow_mut().push(KNOWN_ATTR);
        let mut test_data = RuleData::from_test_data(&test_data);

        no_unknown_attributes(&mut test_data, Severity::Error);
        assert!(test_data.reports.is_empty());
    }
    #[test]
    fn report_unknown_attribute_unknown_element() {
        let mut test_data = RuleData::test_data();
        test_data.element = UNKNOWN_ELEMENT;
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[KNOWN_ATTR, UNKNOWN_ATTR]);
        let mut test_data = RuleData::from_test_data(&test_data);
        no_unknown_attributes(&mut test_data, Severity::Error);
        assert!(test_data.reports.is_empty());
    }
    #[test]
    fn report_unknown_attribute() {
        let mut test_data = RuleData::test_data();
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[KNOWN_ATTR, UNKNOWN_ATTR]);
        test_data.attribute_ranges.insert(
            UNKNOWN_ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);
        no_unknown_attributes(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::UnknownAttribute {
                attribute: UNKNOWN_ATTR_ID,
                element: KNOWN_ELEMENT
            }
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..1));
        assert_eq!(report[0].help, None);
    }
    #[test]
    fn report_unknown_attribute_prefixed() {
        let mut test_data = RuleData::test_data();
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[KNOWN_ATTR, UNKNOWN_PREFIXED_ATTR]);
        test_data.attribute_ranges.insert(
            UNKNOWN_PREFIXED_ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);
        no_unknown_attributes(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::UnknownAttribute {
                attribute: UNKNOWN_PREFIXED_ATTR_ID,
                element: KNOWN_ELEMENT
            }
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..1));
        assert_eq!(
            report[0].help,
            Some(String::from(
                "Unknown prefix defined by `xmlns:xml=\"https://unknown.org\"`"
            ))
        );
    }
}
