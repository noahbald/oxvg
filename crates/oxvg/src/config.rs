//! Types for the configuration file usable by OXVG
use clap::ValueEnum;
use derive_where::derive_where;
use oxvg_ast::element::Element;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Deserialize, Serialize, Debug, Default, Clone, ValueEnum)]
#[serde(rename_all = "camelCase")]
/// A preset which the specified jobs can overwrite
pub enum Extends {
    /// A preset that contains no jobs.
    None,
    #[default]
    /// The default preset.
    /// Uses [`oxvg_optimiser::Jobs::default`]
    Default,
    /// The correctness preset. Produces a preset that is less likely to
    /// visually change the document.
    /// Uses [`oxvg_optimiser::Jobs::correctness`]
    Safe,
    // TODO: File(Path),
}

#[derive_where(Debug, Default)]
#[derive(Serialize, Deserialize)]
#[serde(bound = "E: Element<'arena>")]
/// The configuration for optimisation
pub struct Optimise<'arena, E: Element<'arena>> {
    /// The preset the jobs will extend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<Extends>,
    /// The set of jobs to run
    pub jobs: oxvg_optimiser::Jobs<'arena, E>,
    /// A list of jobs to exclude from running
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omit: Option<Vec<String>>,
}

#[derive_where(Debug, Default)]
#[derive(Deserialize, Serialize)]
#[serde(bound = "E: Element<'arena>")]
#[serde(rename_all = "camelCase")]
/// The config for the CLI usage of OXVG
pub struct Config<'arena, E: Element<'arena>> {
    /// The options for each job to override the specified preset.
    pub optimise: Option<Optimise<'arena, E>>,
}

impl Extends {
    fn jobs<'arena, E: Element<'arena>>(&self) -> oxvg_optimiser::Jobs<'arena, E> {
        match self {
            Extends::None => oxvg_optimiser::Jobs::<E>::none(),
            Extends::Default => oxvg_optimiser::Jobs::<E>::default(),
            Extends::Safe => oxvg_optimiser::Jobs::<E>::safe(),
        }
    }

    /// Creates a configuration with the presets jobs extended by the given jobs.
    pub fn extend<'arena, E: Element<'arena>>(
        &self,
        jobs: &oxvg_optimiser::Jobs<'arena, E>,
    ) -> oxvg_optimiser::Jobs<'arena, E> {
        let mut result = self.jobs();
        result.extend(jobs);
        result
    }
}

impl<'arena, E: Element<'arena>> Optimise<'arena, E> {
    /// Creates a job configuration where the user-configured jobs extends the preset
    /// specified in `extends`
    pub fn resolve_jobs(&self) -> oxvg_optimiser::Jobs<'arena, E> {
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
    let config: Optimise<oxvg_ast::implementations::shared::Element> = serde_json::from_str(
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
            ..oxvg_optimiser::Jobs::<oxvg_ast::implementations::shared::Element>::none()
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
