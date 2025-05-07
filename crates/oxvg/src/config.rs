//! Types for the configuration file usable by OXVG
use oxvg_optimiser::Extends;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
/// The configuration for optimisation
pub struct Optimise {
    /// The preset the jobs will extend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<Extends>,
    /// The set of jobs to run
    pub jobs: oxvg_optimiser::Jobs,
    /// A list of jobs to exclude from running
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omit: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
/// The config for the CLI usage of OXVG
pub struct Config {
    /// The options for each job to override the specified preset.
    pub optimise: Option<Optimise>,
}

impl Optimise {
    /// Creates a job configuration where the user-configured jobs extends the preset
    /// specified in `extends`
    pub fn resolve_jobs(&self) -> oxvg_optimiser::Jobs {
        let Some(extends) = &self.extends else {
            return self.jobs.clone();
        };
        let mut result = extends.extend(&self.jobs);
        if let Some(omit) = self.omit.as_ref() {
            for omit in omit {
                result.omit(omit);
            }
        }
        result
    }
}

#[test]
fn serde() -> anyhow::Result<()> {
    let config: Optimise = serde_json::from_str(
        r#"{
        "extends": "default",
        "jobs": {
            "removeComments": {}
        }
    }"#,
    )?;

    assert_eq!(
        serde_json::to_string(&config)?,
        String::from(r#"{"extends":"default","jobs":{"removeComments":{}}}"#),
        "preset should not affect deserialization of `jobs`"
    );

    Ok(())
}

#[test]
#[allow(clippy::default_trait_access)]
fn resolve_jobs() {
    let config = Optimise {
        extends: Some(Extends::Default),
        jobs: oxvg_optimiser::Jobs {
            precheck: Some(Default::default()),
            remove_scripts: Some(Default::default()),
            ..oxvg_optimiser::Jobs::none()
        },
        omit: Some(vec![
            String::from("precheck"),
            String::from("remove_doctype"),
        ]),
    };

    let resolved = config.resolve_jobs();
    assert!(resolved.precheck.is_none(), "ommited value should be None");
    assert!(
        resolved.remove_doctype.is_none(),
        "ommited value shoud be None"
    );
    assert!(
        resolved.remove_scripts.is_some(),
        "specified value should be Some"
    );
    assert!(
        resolved.remove_comments.is_some(),
        "extended value should be Some"
    );
}
