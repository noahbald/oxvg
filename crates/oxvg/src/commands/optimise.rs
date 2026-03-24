use std::path::PathBuf;

use anyhow::anyhow;
use oxvg_ast::{
    parse::roxmltree::parse_with_options,
    visitor::Info,
    xmlwriter::{Indent, Options, Space},
};
use oxvg_optimiser::{Extends, Jobs};
use roxmltree::ParsingOptions;

use crate::{
    args::RunCommand,
    config::{self, Config},
    walk::{Output, Walk},
};

#[derive(clap::Args, Debug)]
/// Runs optimisation tasks against the given SVG documents.
pub struct Optimise {
    #[clap(flatten)]
    /// Walk options
    pub walk: Walk,
    /// A path to the specified config.
    /// If no config is specified the current config will be printed instead.
    #[clap(long, short, num_args(0..=1))]
    pub config: Option<Vec<PathBuf>>,
    #[clap(long, short)]
    /// When running without a config, sets the default preset to run with
    pub extends: Option<Extends>,
    /// Controls whether the output is indented with tabs or spaces.
    ///
    /// Accepts `none`, `tabs`, or a number
    #[clap(long, short, default_value = "none")]
    pub pretty: Indent,
    /// Controls how the output handles whitespace.
    #[clap(long, short, default_value = "auto")]
    pub space: Space,
}

impl RunCommand for Optimise {
    async fn run(self, config: Config) -> anyhow::Result<()> {
        let config = self.handle_config(config)?;
        let Some(config) = config else {
            return Ok(());
        };

        let jobs = config
            .optimise
            .as_ref()
            .map(config::Optimise::resolve_jobs)
            .unwrap_or_default();
        self.walk(jobs)
    }
}

impl Optimise {
    /// Sets up directory walker and uses it to run the given jobs on each file.
    ///
    /// # Errors
    ///
    /// When invalid options are given
    pub fn walk(self, jobs: Jobs) -> anyhow::Result<()> {
        self.walk.run(move || {
            let jobs = jobs.clone();
            let format_options = Options {
                indent: self.pretty,
                trim_whitespace: self.space,
                ..Options::default()
            };
            Box::new(move |source, path, output| {
                let result = parse_with_options(
                    source,
                    ParsingOptions {
                        allow_dtd: true,
                        ..ParsingOptions::default()
                    },
                    #[allow(clippy::cast_precision_loss)]
                    |dom, allocator| -> anyhow::Result<()> {
                        let input_bytes = source.len() as f64;
                        let info = Info {
                            path: path.cloned(),
                            multipass_count: 0,
                            allocator,
                        };
                        jobs.run(dom, &info)
                            .map_err(|e| anyhow::Error::msg(e.to_string()))?;

                        let output = Output {
                            options: format_options,
                            dom,
                            input: path,
                            destination: output,
                            input_bytes,
                        };
                        output.output()?;
                        Ok(())
                    },
                );
                match result {
                    Err(err) => eprintln!("{err}"),
                    Ok(Err(err)) => eprintln!("{err}"),
                    Ok(Ok(())) => {}
                }
            })
        })
    }

    fn handle_config(&self, config: Config) -> anyhow::Result<Option<Config>> {
        if let Some(config_paths) = &self.config {
            if let Some(config_path) = config_paths.first() {
                log::debug!("using specified config");
                let file = std::fs::File::open(config_path)?;
                serde_json::from_reader(file)
                    .map_err(|e| anyhow!(e))
                    .map(Some)
            } else {
                log::debug!("printing config");
                serde_json::to_writer(
                    std::io::stdout(),
                    &Config {
                        optimise: Some(config.optimise.unwrap_or_default()),
                        lint: Some(config.lint.unwrap_or_default()),
                    },
                )?;
                Ok(None)
            }
        } else if let Some(extends) = &self.extends {
            Ok(Some(Config {
                optimise: Some(crate::config::Optimise {
                    extends: Some(extends.clone()),
                    jobs: Jobs::none(),
                    omit: None,
                }),
                lint: config.lint,
            }))
        } else {
            log::debug!("using inferred config");
            Ok(Some(config))
        }
    }
}
