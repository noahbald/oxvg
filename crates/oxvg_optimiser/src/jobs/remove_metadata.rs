use oxvg_ast::{element::Element, name::Name, node::Node};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Job, JobDefault};

use super::PrepareOutcome;

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMetadata(bool);

impl Job for RemoveMetadata {
    fn prepare<N: Node>(&mut self, _document: &N) -> super::PrepareOutcome {
        if self.0 {
            PrepareOutcome::None
        } else {
            PrepareOutcome::Skip
        }
    }

    fn run<E: Element>(&self, element: &E, _context: &super::Context) {
        let name = element.qual_name();
        if name.prefix().is_some() {
            return;
        }

        if name.local_name() == "metadata".into() {
            element.remove();
        }
    }
}

impl Default for RemoveMetadata {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn remove_metadata() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeMetadata": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <metadata>...</metadata>
    <g/>
</svg>"#
        ),
    )?);

    Ok(())
}
