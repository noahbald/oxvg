//! Types for the configuration file usable by OXVG
use std::{env::current_dir, fs::read_to_string, path::PathBuf};

use etcetera::{choose_base_strategy, BaseStrategy};
use oxvg_lint::Rules;
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
    /// The options for each lint to override the default configuration.
    pub lint: Option<Rules>,
}

impl Config {
    fn load_local() -> std::io::Result<(String, PathBuf)> {
        let mut path = current_dir()?;
        path.push("oxvgrc.json");
        Ok((read_to_string(&path)?, path))
    }

    fn load_base() -> std::io::Result<(String, PathBuf)> {
        let mut path = choose_base_strategy()
            .unwrap_or_else(|err| panic!("{err}"))
            .config_dir();
        path.push("oxvg");
        path.push("config.json");
        Ok((read_to_string(&path)?, path))
    }

    /// Tries loading the configuration from well-known paths
    ///
    /// # Errors
    /// When the config is missing
    ///
    /// # Panics
    /// When the config exists but cannot be parsed
    pub fn load() -> std::io::Result<Self> {
        let (file, path) = Self::load_local().or_else(|_| Self::load_base())?;
        Ok(serde_json::from_str(&file).unwrap_or_else(|err| {
            panic!(
                "Configuration at {} cannot be parsed: {err}",
                path.to_string_lossy()
            )
        }))
    }
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
        String::from(r#"{"extends":"default","jobs":[{"name":"removeComments","params":{}}]}"#),
        "preset should not affect deserialization of `jobs`"
    );

    Ok(())
}

#[test]
#[allow(clippy::default_trait_access)]
fn resolve_jobs() {
    let config = Optimise {
        extends: Some(Extends::Default),
        jobs: oxvg_optimiser::Jobs(vec![
            Box::new(oxvg_optimiser::Precheck::default()),
            Box::new(oxvg_optimiser::RemoveDoctype::default()),
        ]),
        omit: Some(vec![
            String::from("precheck"),
            String::from("remove_doctype"),
        ]),
    };

    let resolved = config.resolve_jobs();
    assert_eq!(resolved.0.len(), 43);
    assert!(!resolved.0.iter().any(|j| j.name() == "precheck"));
    assert!(!resolved.0.iter().any(|j| j.name() == "remove_doctype"));
}
