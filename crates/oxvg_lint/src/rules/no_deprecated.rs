use std::collections::HashMap;

use oxvg_ast::node::Ranges;
use oxvg_collections::{
    attribute::{Attr, AttrId, AttributeInfo},
    element::{ElementId, ElementInfo},
};
use rayon::prelude::*;

use crate::error::{DeprecatedProblem, Error, Problem};

use super::Severity;

pub fn no_deprecated<'a, 'input>(
    element: &'a ElementId<'input>,
    attributes: &'a [Attr<'input>],
    range: Option<&'a std::ops::Range<usize>>,
    attribute_ranges: &'a HashMap<AttrId<'input>, Ranges>,
    severity: Severity,
) -> impl ParallelIterator<Item = Error<'input>> + use<'a, 'input> {
    let once = if element.info().intersects(ElementInfo::Legacy) {
        rayon::iter::once(Some(Error {
            problem: Problem::Deprecated(DeprecatedProblem::DeprecatedElement(element.clone())),
            severity,
            range: range.map(|range| range.start..range.start),
            help: None,
        }))
    } else {
        rayon::iter::once(None)
    }
    .filter_map(|e| e);

    once.chain(attributes.par_iter().filter_map(move |attr| {
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
        Severity,
    };
    use oxvg_ast::node::Ranges;
    use oxvg_collections::{
        atom::Atom,
        attribute::{uncategorised::ViewBox, xml::XmlSpace, Attr, AttrId},
        element::ElementId,
    };
    use rayon::iter::ParallelIterator as _;
    use std::collections::HashMap;

    static OK_ELEMENT: ElementId = ElementId::Svg;
    static LEGACY_ELEMENT: ElementId = ElementId::TRef;
    static OK_ATTRIBUTE: Attr = Attr::ViewBox(ViewBox {
        min_x: 0.0,
        min_y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    static DEPRECATED_SAFE_ATTRIBUTE: Attr = Attr::Version(Atom::Static("1.1"));
    static DEPRECATED_UNSAFE_ATTRIBUTE: Attr = Attr::XmlSpace(XmlSpace::Default);
    static DEPRECATED_SAFE_ATTR_ID: AttrId = AttrId::Version;
    static DEPRECATED_UNSAFE_ATTR_ID: AttrId = AttrId::XmlSpace;

    #[test]
    fn report_deprecated_ok() {
        let ranges = HashMap::new();
        let report: Vec<_> = no_deprecated(
            &OK_ELEMENT,
            &[OK_ATTRIBUTE.clone()],
            None,
            &ranges,
            Severity::Error,
        )
        .collect();
        assert!(report.is_empty());
    }
    #[test]
    fn report_deprecated_element() {
        let ranges = HashMap::new();
        let report: Vec<_> = no_deprecated(
            &LEGACY_ELEMENT,
            &[],
            Some(&(0..1)),
            &ranges,
            Severity::Error,
        )
        .collect();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::Deprecated(DeprecatedProblem::DeprecatedElement(LEGACY_ELEMENT.clone()))
        );
        assert_eq!(report[0].severity, Severity::Error);
        assert_eq!(report[0].range, Some(0..0));
        assert_eq!(report[0].help, None);
    }
    #[test]
    fn report_deprecated_attribute_safe() {
        let ranges = HashMap::from([(
            DEPRECATED_SAFE_ATTR_ID.clone(),
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        )]);
        let report: Vec<_> = no_deprecated(
            &OK_ELEMENT,
            &[DEPRECATED_SAFE_ATTRIBUTE.clone()],
            None,
            &ranges,
            Severity::Error,
        )
        .collect();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report[0].problem,
            Problem::Deprecated(DeprecatedProblem::DeprecatedAttribute(
                DEPRECATED_SAFE_ATTR_ID.clone()
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
        let ranges = HashMap::from([(
            DEPRECATED_UNSAFE_ATTR_ID.clone(),
            Ranges {
                range: 0..1,
                name: 1..2,
                value: 2..3,
            },
        )]);
        let report: Vec<_> = no_deprecated(
            &OK_ELEMENT,
            &[DEPRECATED_UNSAFE_ATTRIBUTE.clone()],
            None,
            &ranges,
            Severity::Error,
        )
        .collect();
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
