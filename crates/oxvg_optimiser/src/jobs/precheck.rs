use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node::{self, Node},
    visitor::Visitor,
};
use oxvg_collections::collections::{ANIMATION_EVENT, DOCUMENT_EVENT, GRAPHICAL_EVENT};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
/// Runs a series of checks to more confidently be sure the document won't break
/// due to unsupported/unstable features.
pub struct Precheck {
    /// Whether to exit with an error instead of a log
    #[serde(default = "default_fail_fast")]
    fail_fast: bool,
    /// Whether to run thorough pre-clean checks as to maintain document correctness
    /// similar to [svgcleaner](https://github.com/RazrFalcon/svgcleaner)
    #[serde(default = "default_preclean_check")]
    preclean_checks: bool,
}

impl<E: Element> Visitor<E> for Precheck {
    type Error = String;

    fn document(
        &mut self,
        document: &mut E,
        _context: &oxvg_ast::visitor::Context<E>,
    ) -> Result<(), Self::Error> {
        document.try_for_each_child(|child| {
            if child.node_type() != node::Type::Comment {
                return Ok(());
            }
            let Some(content) = child.text_content() else {
                return Ok(());
            };
            if content.starts_with("ENTITY ") {
                self.emit::<E>(Self::DTD_ENTITY_ERROR)?;
            }
            Ok(())
        })
    }

    fn element(
        &mut self,
        element: &mut E,
        _context: &mut oxvg_ast::visitor::Context<E>,
    ) -> Result<(), Self::Error> {
        self.qual_name::<E>(element.qual_name())?;

        for attr in element.attributes().into_iter() {
            self.qual_name::<E>(attr.name())?;
        }

        if self.preclean_checks {
            self.check_for_unsupported_elements(element)?;
            self.check_for_script_attributes(element)?;
            self.check_for_conditional_attributes(element)?;
            self.check_for_external_xlink(element)?;
        }

        Ok(())
    }
}

impl Precheck {
    const DTD_ENTITY_ERROR: &str =
        "Document appears to contain DTD Entity, which will be stripped by the parser";
    const XMLNS_PREFIX_ERROR: &str =
        "Document uses an xmlns prefixed element, which may be moved by the parser";
    const NS_UNSTABLE_ERROR: &str =
        "Document uses a namespace which may have been removed by the parser";

    fn qual_name<E: Element>(&self, name: &E::Name) -> Result<(), <Self as Visitor<E>>::Error> {
        if let Some(prefix) = name.prefix() {
            if prefix.as_ref() == "xmlns" {
                self.emit::<E>(Self::XMLNS_PREFIX_ERROR)?;
            }
        }

        let ns = name.ns().as_ref();
        if ns.is_empty() {
            self.emit::<E>(Self::NS_UNSTABLE_ERROR)?;
        }

        Ok(())
    }

    fn emit<E: Element>(&self, message: &str) -> Result<(), <Self as Visitor<E>>::Error> {
        if self.fail_fast {
            Err(message.to_string())
        } else {
            log::error!("{message}");
            Ok(())
        }
    }
}

// NOTE: The following ports code from svgcleaner and thus this inherits the license
// https://github.com/RazrFalcon/svgcleaner/blob/master/LICENSE.txt
impl Precheck {
    const SCRIPTING_NOT_SUPPORTED: &str = "scripting is not supported";
    const ANIMATION_NOT_SUPPORTED: &str = "animation is not supported";
    const CONDITIONAL_NOT_SUPPORTED: &str = "conditional processing attributes is not supported";

    fn check_for_unsupported_elements<E: Element>(
        &self,
        element: &E,
    ) -> Result<(), <Self as Visitor<E>>::Error> {
        if element.prefix().is_some() {
            return Ok(());
        }

        match element.local_name().as_ref() {
            "script" => self.emit::<E>(Self::SCRIPTING_NOT_SUPPORTED),
            "animate" | "animateColor" | "animateMotion" | "animateTransform" | "set" => {
                self.emit::<E>(Self::ANIMATION_NOT_SUPPORTED)
            }
            _ => Ok(()),
        }
    }

    fn check_for_script_attributes<E: Element>(
        &self,
        element: &E,
    ) -> Result<(), <Self as Visitor<E>>::Error> {
        for attr in element.attributes().into_iter() {
            if attr.name().prefix().is_some() {
                continue;
            }

            let local_name = attr.name().local_name();
            if GRAPHICAL_EVENT.contains(local_name)
                || DOCUMENT_EVENT.contains(local_name)
                || ANIMATION_EVENT.contains(local_name)
            {
                self.emit::<E>(Self::SCRIPTING_NOT_SUPPORTED)?;
            }
        }

        Ok(())
    }

    fn check_for_conditional_attributes<E: Element>(
        &self,
        element: &E,
    ) -> Result<(), <Self as Visitor<E>>::Error> {
        for attr in element.attributes().into_iter() {
            if attr.name().prefix().is_some() {
                continue;
            }
            if attr.value().is_empty() {
                continue;
            }

            match attr.name().local_name().as_ref() {
                "requiredFeatures" | "systemLanguage" => {
                    self.emit::<E>(Self::CONDITIONAL_NOT_SUPPORTED)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn check_for_external_xlink<E: Element>(
        &self,
        element: &E,
    ) -> Result<(), <Self as Visitor<E>>::Error> {
        if matches!(
            element.local_name().as_ref(),
            "a" | "image" | "font-face-uri" | "feImage"
        ) {
            return Ok(());
        }

        for attr in element.attributes().into_iter() {
            if attr
                .name()
                .prefix()
                .as_ref()
                .is_none_or(|p| p.as_ref() != "xlink")
            {
                continue;
            }
            if attr.name().local_name().as_ref() != "href" {
                continue;
            }

            self.emit::<E>(&format!("the `xlink:href` attribute is referencing an external object '{}' which is not supported", attr.value()))?;
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

    // NOTE: First entity (st1) is omitted by parser
    // NOTE: Entities are converted into comments
    assert_eq!(
        &test_config(
            r#"{ "precheck": { "failFast": true } }"#,
            Some(
            r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd" [<!ENTITY st1 "opacity:.34;"><!ENTITY st2 "fill:#231F20;">]>
<!-- emit error for DTD Entities -->
<svg version="1.1" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" width="103px" height="94px" viewBox="0 0 103 94">
    <path style="&st2;" d="M70.903,83.794c0.2,0.064,0.106,0.171-0.087,0.278l0.089-0.033c0,0,0.793-0.436,0.458-0.633c-0.338-0.198-1.129-0.275-0.613-0.969l0.02-0.02c-0.442,0.344-0.756,0.727-0.498,1.021C70.27,83.443,70.387,83.629,70.903,83.794z"/>
</svg>"#,
            ),
        ).unwrap_err().to_string(),
        Precheck::DTD_ENTITY_ERROR
    );

    assert_eq!(
        &test_config(
            r#"{ "precheck": { "failFast": true } }"#,
            Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:sodipodi="http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd">
    <!-- emit error for xmlns prefix -->
    <path d="..." sodipodi:nodetypes="cccc"/>
</svg>"#,
            ),
        ).unwrap_err().to_string(),
        Precheck::NS_UNSTABLE_ERROR
    );

    // FIXME: uncomment when xmlns is stable
    //     assert_eq!(
    //         &test_config(
    //             r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
    //             Some(
    //             r##"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
    //     <!-- emit error for animation element -->
    //     <circle id="1" cx="5.5" cy="5.5" r="5.5">
    //         <animate attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
    //     </circle>
    // </svg>"##,
    //             ),
    //         ).unwrap_err().to_string(),
    //         Precheck::ANIMATION_NOT_SUPPORTED
    //     );

    // FIXME: uncomment when xmlns is stable
    //     assert_eq!(
    //         &test_config(
    //             r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
    //             Some(
    //             r#"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
    //     <!-- emit error for script attribute -->
    //     <circle id="1" cx="5.5" cy="5.5" r="5.5" onerror="function (error) { console.log(error) }" />
    // </svg>"#,
    //             ),
    //         ).unwrap_err().to_string(),
    //         Precheck::SCRIPTING_NOT_SUPPORTED
    //     );

    // FIXME: uncomment when xmlns is stable
    //     assert_eq!(
    //         &test_config(
    //             r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
    //             Some(
    //             r#"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
    //     <!-- emit error for script attribute -->
    //     <circle id="1" cx="5.5" cy="5.5" r="5.5" requiredFeatures="http://www.w3.org/TR/SVG11/feature#SVG" />
    // </svg>"#,
    //             ),
    //         ).unwrap_err().to_string(),
    //         Precheck::CONDITIONAL_NOT_SUPPORTED
    //     );

    // FIXME: uncomment when xmlns is stable
    //     let _ = test_config(
    //         r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
    //         Some(
    //         r#"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg">
    // <!-- emit error for script attribute -->
    // <circle id="1" cx="5.5" cy="5.5" r="5.5" requiredFeatures="" />
    // </svg>"#,
    //         ),
    //     ).unwrap();

    // FIXME: uncomment when xmlns is stable
    //     assert_eq!(
    //         &test_config(
    //             r#"{ "precheck": { "failFast": true, "precleanChecks": true } }"#,
    //             Some(
    //             r##"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    //     <!-- emit error for script attribute -->
    //     <a xlink:href="#uwu" />
    // </svg>"##,
    //             ),
    //         ).unwrap_err().to_string(),
    //         "the `xlink:href` attribute is referencing an external object '#uwu' which is not supported"
    //     );
}
