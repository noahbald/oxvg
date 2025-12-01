use oxvg_collections::{element::ElementId, is_prefix};

use crate::{
    error::{Error, Problem},
    utils::prefix_help,
};

use super::Severity;

pub fn no_unknown_elements<'input>(
    parent: Option<&ElementId<'input>>,
    name: &ElementId<'input>,
    range: Option<&std::ops::Range<usize>>,
    severity: Severity,
) -> Option<Error<'input>> {
    let parent = parent?;
    if !is_prefix!(name.prefix(), SVG) {
        return None;
    }

    if parent.is_permitted_child(name) {
        return None;
    }
    Some(Error {
        problem: Problem::UnknownElement {
            parent: parent.clone(),
            element: name.clone(),
        },
        severity,
        // NOTE: roxmltree range spans from the start of the opening-tag to the end of the closing-tag
        //       Using a zero-len range will cause the error reporter to use `utils::naive_range`.
        range: range.map(|range| range.start..range.start),
        help: prefix_help(name.prefix()),
    })
}

#[cfg(test)]
mod test {
    use super::no_unknown_elements;
    use crate::{error::Problem, Severity};
    use oxvg_collections::element::ElementId;

    static PARENT_ELEMENT: Option<ElementId<'static>> = Some(ElementId::Svg);
    static KNOWN_ELEMENT: ElementId<'static> = ElementId::Circle;
    static UNKNOWN_ELEMENT: ElementId<'static> = ElementId::Stop;

    #[test]
    fn report_unknown_element_ok() {
        let report = no_unknown_elements(
            PARENT_ELEMENT.as_ref(),
            &KNOWN_ELEMENT,
            None,
            Severity::Error,
        );
        assert!(report.is_none());
    }
    #[test]
    fn report_unknown_element() {
        let report = no_unknown_elements(
            PARENT_ELEMENT.as_ref(),
            &UNKNOWN_ELEMENT,
            Some(&(0..1)),
            Severity::Error,
        )
        .unwrap();
        assert_eq!(
            report.problem,
            Problem::UnknownElement {
                parent: PARENT_ELEMENT.clone().unwrap(),
                element: UNKNOWN_ELEMENT.clone(),
            }
        );
        assert_eq!(report.severity, Severity::Error);
        assert_eq!(report.range, Some(0..0));
        assert_eq!(report.help, None);
    }
}
