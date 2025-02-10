use std::{
    ffi::OsStr,
    io::{IsTerminal, Read, Write},
    path::PathBuf,
    str::FromStr,
};

use anyhow::anyhow;
use oxvg_ast::{
    implementations::markup5ever::{Element5Ever, Node5Ever},
    visitor::Info,
};
use oxvg_optimiser::Jobs;
use walkdir::WalkDir;

use crate::{args::RunCommand, config::Config};

#[derive(clap::Args)]
pub struct Optimise {
    /// The target paths to optimise
    #[clap(value_parser)]
    pub paths: Vec<PathBuf>,
    /// Whether to write to the specified file or directory.
    /// Will use the input if flag is given without a value.
    /// Defaults to stdout.
    #[clap(long, short, num_args(0..=1))]
    pub output: Option<Vec<PathBuf>>,
    /// A path to the specified config.
    /// If no config is specified the current config will be printed instead.
    #[clap(long, short, num_args(0..=1))]
    pub config: Option<Vec<PathBuf>>,
    #[clap(long, short, default_value = "false")]
    pub recursive: bool,
}

impl RunCommand for Optimise {
    fn run(&self, config: Config) -> anyhow::Result<()> {
        let config = self.handle_config(config)?;
        let Some(config) = config else {
            return Ok(());
        };
        let jobs = config.optimisation.unwrap_or_default();

        self.handle_paths(&jobs)
    }
}

impl Optimise {
    fn handle_out<W: Write>(dom: &Node5Ever, wr: W) -> anyhow::Result<()> {
        use oxvg_ast::serialize::Node;

        dom.serialize_into(wr)
    }

    fn handle_stdin(&self, jobs: &Jobs<Element5Ever>) -> anyhow::Result<()> {
        use oxvg_ast::parse::Node;

        let mut string = String::new();
        std::io::stdin().read_to_string(&mut string)?;
        let dom = Node5Ever::parse(&string)?;

        let info = Info {
            path: None,
            multipass_count: 0,
        };
        jobs.clone().run(&dom, &info)?;

        if let Some(output) = &self.output.as_ref().and_then(|o| {
            eprintln!("Warning: Using empty `-o,--output` with stdin will print to stdout, you can instead omit `-o,--output`.");
            o.first()
        }) {
            let file = std::fs::File::open(output)?;
            if file.metadata()?.is_file() {
                eprintln!(
                    "Cannot use dir as output for stdin input. Printing result to stdout instead"
                );
                Self::handle_out(&dom, std::io::stdout())?;
            } else {
                Self::handle_out(&dom, file)?;
            }
        } else {
            Self::handle_out(&dom, std::io::stdout())?;
        }

        Ok(())
    }

    fn handle_file(
        jobs: &Jobs<Element5Ever>,
        path: &PathBuf,
        output: Option<&PathBuf>,
    ) -> anyhow::Result<()> {
        use oxvg_ast::parse::Node;

        let file = std::fs::File::open(path)?;
        let dom = Node5Ever::parse_file(&file)?;
        drop(file);

        let info = Info {
            path: Some(path.clone()),
            multipass_count: 0,
        };
        jobs.clone().run(&dom, &info)?;

        match output {
            Some(output_path) => {
                println!("Optimising to file {output_path:?}");
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let file = std::fs::File::create(output_path)?;
                Self::handle_out(&dom, file)
            }
            None => Self::handle_out(&dom, std::io::stdout()),
        }
    }

    fn handle_path(&self, jobs: &Jobs<Element5Ever>, path: &PathBuf) -> anyhow::Result<()> {
        let output_path = |input: &PathBuf| {
            let Some(output) = self.output.as_ref() else {
                return Ok(None);
            };
            let Some(output) = output.first() else {
                return Ok(Some(input.clone()));
            };
            input.strip_prefix(path).map(|p| Some(output.join(p)))
        };
        let mut walker = WalkDir::new(path);
        if !self.recursive {
            walker = walker.max_depth(1);
        }
        for file in walker.follow_links(true) {
            let file = file?;
            if !file.file_type().is_file() {
                continue;
            }
            let path = file.into_path();
            if path.extension().and_then(OsStr::to_str) != Some("svg") {
                continue;
            }
            Self::handle_file(jobs, &path, output_path(&path)?.as_ref())?;
        }

        Ok(())
    }

    fn handle_paths(&self, jobs: &Jobs<Element5Ever>) -> anyhow::Result<()> {
        if !std::io::stdin().is_terminal()
            && self.paths.len() <= 1
            && self
                .paths
                .first()
                .is_none_or(|path| path == &PathBuf::from_str(".").unwrap())
        {
            return self.handle_stdin(jobs);
        }
        if self.paths.is_empty() {
            return Err(anyhow!(
                "`oxvg optimise` requires at least one path to optimise"
            ));
        }

        for path in &self.paths {
            self.handle_path(jobs, path)?;
        }
        Ok(())
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
                serde_json::to_writer(std::io::stdout(), &config.optimisation.unwrap_or_default())?;
                Ok(None)
            }
        } else {
            log::debug!("using inferred config");
            Ok(Some(config))
        }
    }
}
