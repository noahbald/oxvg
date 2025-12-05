use oxvg_collections::is_prefix;

use crate::{
    error::{Error, Problem},
    utils::prefix_help,
};

use super::{RuleData, Severity};

pub fn no_unknown_elements<'input>(
    RuleData {
        reports,
        parent,
        element,
        range,
        ..
    }: &mut RuleData<'_, 'input>,
    severity: Severity,
) {
    let Some(parent) = parent else {
        return;
    };
    if !is_prefix!(element.prefix(), SVG) {
        return;
    }

    if parent.is_permitted_child(element) {
        return;
    }
    reports.push(Error {
        problem: Problem::UnknownElement {
            parent: parent.clone(),
            element: (*element).clone(),
        },
        severity,
        // NOTE: roxmltree range spans from the start of the opening-tag to the end of the closing-tag
        //       Using a zero-length range will cause the error reporter to use `utils::naive_range`.
        range: range.as_ref().map(|range| range.start..range.start),
        help: prefix_help(element.prefix()),
    })
}

#[cfg(test)]
mod test {
    use super::no_unknown_elements;
    use crate::{error::Problem, rules::RuleData, Severity};
    use oxvg_collections::element::ElementId;

    const PARENT_ELEMENT: Option<ElementId<'static>> = Some(ElementId::Svg);
    const KNOWN_ELEMENT: ElementId<'static> = ElementId::Circle;
    const UNKNOWN_ELEMENT: ElementId<'static> = ElementId::Stop;

    #[test]
    fn report_unknown_element_ok() {
        let mut test_data = RuleData::test_data();
        test_data.element = KNOWN_ELEMENT;
        let mut test_data = RuleData::from_test_data(&test_data);
        no_unknown_elements(&mut test_data, Severity::Error);
        assert!(test_data.reports.is_empty());
    }
    #[test]
    fn report_unknown_element() {
        let mut test_data = RuleData::test_data();
        test_data.element = UNKNOWN_ELEMENT;
        let mut test_data = RuleData::from_test_data(&test_data);
        no_unknown_elements(&mut test_data, Severity::Error);
        assert_eq!(test_data.reports.len(), 1);
        let report = test_data.reports.first().unwrap();
        assert_eq!(
            report.problem,
            Problem::UnknownElement {
                parent: PARENT_ELEMENT.unwrap(),
                element: UNKNOWN_ELEMENT,
            }
        );
        assert_eq!(report.severity, Severity::Error);
        assert_eq!(report.range, Some(0..0));
        assert_eq!(report.help, None);
    }
}
