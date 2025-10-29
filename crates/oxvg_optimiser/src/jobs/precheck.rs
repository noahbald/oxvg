/*!
LEGAL: This file is distributed under the GPL v2 license, based on the derived work
of [svgcleaner](https://github.com/RazrFalcon/svgcleaner/blob/master/src/task/preclean_checks.rs).

See [license](https://github.com/RazrFalcon/svgcleaner/blob/master/LICENSE.txt)
*/
use oxvg_ast::{
    attribute::AttributeGroup,
    element::{data::ElementId, Element},
    get_attribute, is_element,
    visitor::Visitor,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::{JobsError, PrecheckError};

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
/// Runs a series of checks to more confidently be sure the document won't break
/// due to unsupported/unstable features.
///
/// # Errors
///
/// When `fail_fast` is given, the job will fail if it finds any content which
/// may cause the document to break with optimisations.
pub struct Precheck {
    /// Whether to exit with an error instead of a log
    #[serde(default = "default_fail_fast")]
    pub fail_fast: bool,
    /// Whether to run thorough pre-clean checks as to maintain document correctness
    /// similar to [svgcleaner](https://github.com/RazrFalcon/svgcleaner)
    #[serde(default = "default_preclean_check")]
    pub preclean_checks: bool,
}

impl<'input, 'arena> Visitor<'input, 'arena> for Precheck {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut oxvg_ast::visitor::Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if self.preclean_checks {
            self.check(element).map_err(JobsError::Precheck);
        }

        Ok(())
    }
}

impl Precheck {
    fn check<'input, 'arena>(
        &self,
        element: &Element<'input, 'arena>,
    ) -> Result<(), PrecheckError<'input>> {
        self.check_for_unsupported_elements(element)?;
        self.check_for_script_attributes(element)?;
        self.check_for_conditional_attributes(element)?;
        self.check_for_external_xlink(element)
    }

    fn check_for_unsupported_elements<'input, 'arena>(
        &self,
        element: &Element<'input, 'arena>,
    ) -> Result<(), PrecheckError<'input>> {
        match element.qual_name().unaliased() {
            ElementId::Script => Err(PrecheckError::ScriptingNotSupported),
            ElementId::Animate
            | ElementId::AnimateColor
            | ElementId::AnimateMotion
            | ElementId::AnimateTransform
            | ElementId::Set => Err(PrecheckError::AnimationNotSupported),
            _ => Ok(()),
        }
    }

    fn check_for_script_attributes<'input, 'arena>(
        &self,
        element: &Element<'input, 'arena>,
    ) -> Result<(), PrecheckError<'input>> {
        for attr in element.attributes().into_iter() {
            if attr
                .name()
                .attribute_group()
                .intersects(AttributeGroup::event())
            {
                return Err(PrecheckError::ScriptingNotSupported);
            }
        }

        Ok(())
    }

    fn check_for_conditional_attributes<'input, 'arena>(
        &self,
        element: &Element<'input, 'arena>,
    ) -> Result<(), PrecheckError<'input>> {
        for attr in element.attributes().into_iter() {
            if attr
                .name()
                .attribute_group()
                .contains(AttributeGroup::ConditionalProcessing)
            {
                return Err(PrecheckError::ConditionalProcessingNotSupported);
            };
        }

        Ok(())
    }

    fn check_for_external_xlink<'input, 'arena>(
        &self,
        element: &Element<'input, 'arena>,
    ) -> Result<(), PrecheckError<'input>> {
        if is_element!(element, A | Image | FontFaceURI | FeImage) {
            return Ok(());
        }

        if let Some(xlink_href) = get_attribute!(element, XLinkHref) {
            return Err(PrecheckError::ReferencesExternalXLink(xlink_href.clone()));
        }

        Ok(())
    }
}

const fn default_fail_fast() -> bool {
    true
}

const fn default_preclean_check() -> bool {
    false
}

#[test]
fn precheck() {
    use crate::test_config;

    assert_eq!(
            test_config(
                r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
                Some(
                r##"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
        <!-- emit error for animation element -->
        <circle id="1" cx="5.5" cy="5.5" r="5.5">
            <animate attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
        </circle>
    </svg>"##,
                ),
            ).unwrap_err().to_string(),
            PrecheckError::AnimationNotSupported.to_string()
        );

    assert_eq!(
            test_config(
                r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
                Some(
                r#"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
        <!-- emit error for script attribute -->
        <circle id="1" cx="5.5" cy="5.5" r="5.5" onerror="function (error) { console.log(error) }" />
    </svg>"#,
                ),
            ).unwrap_err().to_string(),
            PrecheckError::ScriptingNotSupported.to_string()
        );

    assert_eq!(
            test_config(
                r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
                Some(
                r#"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
        <!-- emit error for requiredFeatures attribute -->
        <circle id="1" cx="5.5" cy="5.5" r="5.5" requiredFeatures="http://www.w3.org/TR/SVG11/feature#SVG" />
    </svg>"#,
                ),
            ).unwrap_err().to_string(),
            PrecheckError::ConditionalProcessingNotSupported.to_string()
        );

    let _ = test_config(
            r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
            Some(
            r#"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
    <!-- empty requiredFeatures is fine -->
    <circle id="1" cx="5.5" cy="5.5" r="5.5" requiredFeatures="" />
    </svg>"#,
            ),
        ).unwrap();

    assert_eq!(
            &test_config(
                r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
                Some(
                r##"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
        <!-- emit error for `xlink:href` attribute -->
        <p xlink:href="#uwu" />
    </svg>"##,
                ),
            ).unwrap_err().to_string(),
            "the `xlink:href` attribute is referencing an external object '#uwu' which is not supported"
        );
}
