use oxvg_collections::{
    attribute::{xlink::XLinkShow, Attr},
    is_prefix,
};
use rayon::prelude::*;

use crate::error::{Error, NoXLinkProblem, Problem};

use super::{RuleData, Severity};

pub fn no_xlink<'a, 'input>(
    RuleData {
        reports,
        attributes,
        attribute_ranges,
        ..
    }: &mut RuleData<'_, 'input>,
    severity: Severity,
) {
    reports.par_extend(attributes.par_iter().filter_map(move |attr| {
        let attr_id = attr.name();
        let problem = match attr.unaliased() {
            Attr::XLinkShow(XLinkShow::Replace) => {
                Problem::NoXLink(NoXLinkProblem::XLinkShowReplace)
            }
            Attr::XLinkShow(XLinkShow::New) => Problem::NoXLink(NoXLinkProblem::XLinkShowNew),
            Attr::XLinkTitle(_) => Problem::NoXLink(NoXLinkProblem::XLinkTitle),
            Attr::XLinkHref(_) => Problem::NoXLink(NoXLinkProblem::XLinkHref),
            _ => {
                if is_prefix!(attr_id, XLink) {
                    Problem::NoXLink(NoXLinkProblem::XLinkUnsupported(attr_id.clone()))
                } else {
                    return None;
                }
            }
        };
        Some(Error {
            problem,
            severity,
            range: attribute_ranges
                .get(attr_id)
                .map(|range| range.range.clone()),
            help: None,
        })
    }));
}

#[cfg(test)]
mod test {
    use super::no_xlink;
    use crate::{
        error::{NoXLinkProblem, Problem},
        rules::RuleData,
        Severity,
    };
    use oxvg_ast::node::Ranges;
    use oxvg_collections::{
        atom::Atom,
        attribute::{Attr, AttrId},
        name::{Prefix, QualName},
    };

    const SVG_ATTR: Attr = Attr::Href(Atom::Static("#"));
    const XLINK_ATTR: Attr = Attr::XLinkHref(Atom::Static("#"));
    const UNKNOWN_XLINK_ATTR: Attr = Attr::Unparsed {
        attr_id: AttrId::Unknown(QualName {
            prefix: Prefix::XLink,
            local: Atom::Static("foo"),
        }),
        value: Atom::Static("bar"),
    };
    const XLINK_ATTR_ID: AttrId = AttrId::XLinkHref;
    const UNKNOWN_XLINK_ATTR_ID: AttrId = AttrId::Unknown(QualName {
        prefix: Prefix::XLink,
        local: Atom::Static("foo"),
    });

    #[test]
    fn report_no_xlink_ok() {
        let test_data = RuleData::test_data();
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[SVG_ATTR]);
        let mut test_data = RuleData::from_test_data(&test_data);
        no_xlink(&mut test_data, Severity::Error);
        assert!(test_data.reports.is_empty());
    }

    #[test]
    fn report_no_xlink_known() {
        let mut test_data = RuleData::test_data();
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[XLINK_ATTR]);
        test_data.attribute_ranges.insert(
            XLINK_ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);
        no_xlink(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::NoXLink(NoXLinkProblem::XLinkHref)
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..1));
        assert_eq!(report[0].help, None);
    }

    #[test]
    fn report_no_xlink_unknown() {
        let mut test_data = RuleData::test_data();
        test_data
            .attributes
            .borrow_mut()
            .extend_from_slice(&[UNKNOWN_XLINK_ATTR]);
        test_data.attribute_ranges.insert(
            UNKNOWN_XLINK_ATTR_ID,
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        );
        let mut test_data = RuleData::from_test_data(&test_data);
        no_xlink(&mut test_data, Severity::Error);
        let report = test_data.reports;
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::NoXLink(NoXLinkProblem::XLinkUnsupported(
                UNKNOWN_XLINK_ATTR_ID.clone()
            ))
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..1));
        assert_eq!(report[0].help, None);
    }
}
