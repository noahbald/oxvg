use std::collections::HashMap;

use oxvg_ast::node::Ranges;
use oxvg_collections::{
    attribute::{xlink::XLinkShow, Attr, AttrId},
    is_prefix,
};
use rayon::prelude::*;

use crate::error::{Error, NoXLinkProblem, Problem};

use super::Severity;

pub fn no_xlink<'a, 'input>(
    attributes: &'a [Attr<'input>],
    attribute_ranges: &'a HashMap<AttrId<'input>, Ranges>,
    severity: Severity,
) -> impl ParallelIterator<Item = Error<'input>> + use<'a, 'input> {
    attributes.par_iter().filter_map(move |attr| {
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
    })
}

#[cfg(test)]
mod test {
    use super::no_xlink;
    use crate::{
        error::{NoXLinkProblem, Problem},
        Severity,
    };
    use oxvg_ast::node::Ranges;
    use oxvg_collections::{
        atom::Atom,
        attribute::{Attr, AttrId},
        name::{Prefix, QualName},
    };
    use rayon::iter::ParallelIterator as _;
    use std::collections::HashMap;

    static SVG_ATTR: Attr = Attr::Href(Atom::Static("#"));
    static XLINK_ATTR: Attr = Attr::XLinkHref(Atom::Static("#"));
    static UNKNOWN_XLINK_ATTR: Attr = Attr::Unparsed {
        attr_id: AttrId::Unknown(QualName {
            prefix: Prefix::XLink,
            local: Atom::Static("foo"),
        }),
        value: Atom::Static("bar"),
    };
    static XLINK_ATTR_ID: AttrId = AttrId::XLinkHref;
    static UNKNOWN_XLINK_ATTR_ID: AttrId = AttrId::Unknown(QualName {
        prefix: Prefix::XLink,
        local: Atom::Static("foo"),
    });

    #[test]
    fn report_no_xlink_ok() {
        let attrs = [SVG_ATTR.clone()];
        let ranges = HashMap::new();
        let report: Vec<_> = no_xlink(&attrs, &ranges, Severity::Error).collect();
        assert_eq!(report.len(), 0);
    }

    #[test]
    fn report_no_xlink_known() {
        let attrs = [XLINK_ATTR.clone()];
        let ranges = HashMap::from([(
            XLINK_ATTR_ID.clone(),
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        )]);
        let report: Vec<_> = no_xlink(&attrs, &ranges, Severity::Error).collect();
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
        let attrs = [UNKNOWN_XLINK_ATTR.clone()];
        let ranges = HashMap::from([(
            UNKNOWN_XLINK_ATTR_ID.clone(),
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        )]);
        let report: Vec<_> = no_xlink(&attrs, &ranges, Severity::Error).collect();
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
