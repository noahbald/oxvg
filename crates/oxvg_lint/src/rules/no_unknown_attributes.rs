use std::collections::HashMap;

use oxvg_ast::node::Ranges;
use oxvg_collections::{
    attribute::{Attr, AttrId},
    element::ElementId,
    name::Prefix,
};
use rayon::prelude::*;

use crate::{
    error::{Error, Problem},
    utils::prefix_help,
};

use super::Severity;

pub fn no_unknown_attributes<'a, 'input>(
    element: &'a ElementId<'input>,
    attributes: &'a [Attr<'input>],
    attribute_ranges: &'a HashMap<AttrId<'input>, Ranges>,
    severity: Severity,
) -> Option<impl ParallelIterator<Item = Error<'input>> + use<'a, 'input>> {
    if matches!(element, ElementId::Unknown(_)) {
        return None;
    }

    let expected_attributes = element.expected_attributes();
    let expected_attribute_groups = element.expected_attribute_groups();
    Some(attributes.par_iter().filter_map(move |attr| {
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
    use crate::{error::Problem, Severity};
    use oxvg_ast::node::Ranges;
    use oxvg_collections::{
        atom::Atom,
        attribute::{Attr, AttrId},
        element::ElementId,
        name::{Prefix, QualName, NS},
    };
    use rayon::iter::ParallelIterator as _;
    use std::collections::HashMap;

    static UNKNOWN_ELEMENT: ElementId<'static> = ElementId::Unknown(QualName {
        prefix: Prefix::SVG,
        local: Atom::Static("foo"),
    });
    static KNOWN_ELEMENT: ElementId<'static> = ElementId::Svg;
    static UNKNOWN_ATTR_ID: AttrId<'static> = AttrId::Unknown(QualName {
        prefix: Prefix::SVG,
        local: Atom::Static("foo"),
    });
    static UNKNOWN_ATTR: Attr<'static> = Attr::Unparsed {
        attr_id: AttrId::Unknown(QualName {
            prefix: Prefix::SVG,
            local: Atom::Static("foo"),
        }),
        value: Atom::Static("bar"),
    };
    static UNKNOWN_PREFIXED_ATTR_ID: AttrId<'static> = AttrId::Unknown(QualName {
        prefix: Prefix::Unknown {
            prefix: Some(Atom::Static("xml")),
            ns: NS::Unknown(Atom::Static("https://unknown.org")),
        },
        local: Atom::Static("foo"),
    });
    static UNKNOWN_PREFIXED_ATTR: Attr<'static> = Attr::Unparsed {
        attr_id: AttrId::Unknown(QualName {
            prefix: Prefix::Unknown {
                prefix: Some(Atom::Static("xml")),
                ns: NS::Unknown(Atom::Static("https://unknown.org")),
            },
            local: Atom::Static("foo"),
        }),
        value: Atom::Static("bar"),
    };
    static KNOWN_ATTR: Attr<'static> = Attr::Version(Atom::Static("1.1"));

    #[test]
    fn report_unknown_attribute_ok() {
        let attrs = [KNOWN_ATTR.clone()];
        let ranges = HashMap::new();
        let report: Vec<_> =
            no_unknown_attributes(&KNOWN_ELEMENT, &attrs, &ranges, Severity::Error)
                .unwrap()
                .collect();
        assert_eq!(report.len(), 0);
    }
    #[test]
    fn report_unknown_attribute_unknown_element() {
        let attrs = [KNOWN_ATTR.clone(), UNKNOWN_ATTR.clone()];
        let ranges = HashMap::new();
        let report = no_unknown_attributes(&UNKNOWN_ELEMENT, &attrs, &ranges, Severity::Error);
        assert!(report.is_none());
    }
    #[test]
    fn report_unknown_attribute() {
        let attrs = [KNOWN_ATTR.clone(), UNKNOWN_ATTR.clone()];
        let ranges = HashMap::from([(
            UNKNOWN_ATTR_ID.clone(),
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        )]);
        let report: Vec<_> =
            no_unknown_attributes(&KNOWN_ELEMENT, &attrs, &ranges, Severity::Error)
                .unwrap()
                .collect();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::UnknownAttribute {
                attribute: UNKNOWN_ATTR_ID.clone(),
                element: KNOWN_ELEMENT.clone()
            }
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..1));
        assert_eq!(report[0].help, None);
    }
    #[test]
    fn report_unknown_attribute_prefixed() {
        let attrs = [KNOWN_ATTR.clone(), UNKNOWN_PREFIXED_ATTR.clone()];
        let ranges = HashMap::from([(
            UNKNOWN_PREFIXED_ATTR_ID.clone(),
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        )]);
        let report: Vec<_> =
            no_unknown_attributes(&KNOWN_ELEMENT, &attrs, &ranges, Severity::Error)
                .unwrap()
                .collect();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::UnknownAttribute {
                attribute: UNKNOWN_PREFIXED_ATTR_ID.clone(),
                element: KNOWN_ELEMENT.clone()
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
