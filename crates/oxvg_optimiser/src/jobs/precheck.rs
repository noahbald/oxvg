use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node::{self, Node},
    visitor::Visitor,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
/// Runs a series of checks to more confidently be sure the document won't break
/// due to unsupported/unstable features.
pub struct Precheck {
    /// Whether to exit with an error instead of a log
    fail_fast: bool,
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
}
