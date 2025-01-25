use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Parser, Subcommand};
use oxvg_ast::{serialize::Node, visitor::Info};

use crate::config::Config;

pub trait RunCommand {
    /// # Errors
    ///
    /// If any part of the lifecycle fails
    /// * Fails to read or parse any files
    /// * Fails to write or serialize to any files
    fn run(&self, config: Config) -> anyhow::Result<()>;
}

#[derive(Parser)]
#[clap(
    bin_name = "oxvg",
    name="oxvg",
    author,
    version,
    about = "Your versatile vector-graphics toolchain",
    long_about = None
)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Optimise SVG documents
    #[clap(alias = "optimize")]
    Optimise(Optimise),
}

#[derive(clap::Args)]
pub struct Optimise {
    /// The target paths to optimise
    #[clap(value_parser, default_value = ".")]
    pub paths: Vec<PathBuf>,
    /// The file or directory to output results to.
    /// Defaults to stdout
    #[clap(long = "output", short = 'o')]
    pub output: Option<PathBuf>,
}

struct StdoutCounter {
    stdout: std::io::Stdout,
    count: usize,
}

impl StdoutCounter {
    fn new() -> Self {
        Self {
            stdout: std::io::stdout(),
            count: 0,
        }
    }
}

impl std::io::Write for StdoutCounter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = self.stdout.write(buf);
        if let Ok(n) = result {
            self.count += n;
        }
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdout.flush()
    }
}

impl RunCommand for Optimise {
    fn run(&self, config: Config) -> anyhow::Result<()> {
        use oxvg_ast::{implementations::markup5ever::Node5Ever, parse::Node};

        if self.paths.len() == 1 {
            let path = self.paths.first().unwrap();
            let file = std::fs::File::open(path)?;
            let dom = Node5Ever::parse_file(&file)?;
            let jobs = config.optimisation.unwrap_or_default();

            let start_time = SystemTime::now().duration_since(UNIX_EPOCH)?;
            let prev_file_size = file.metadata()?.len();

            let info = Info {
                path: Some(path.clone()),
                multipass_count: 0,
            };
            jobs.run(&dom, &info)?;
            let mut stdout = StdoutCounter::new();
            dom.serialize_into(&mut stdout)?;

            let result_file_size = stdout.count;
            let end_time = SystemTime::now().duration_since(UNIX_EPOCH)?;
            let duration = end_time - start_time;

            log::info!("Done in {duration:?}!");
            log::info!(
                "{}.{:#1} KiB -> {}.{:#1} KiB",
                prev_file_size / 1000,
                prev_file_size % 1000,
                result_file_size / 1000,
                result_file_size % 1000
            );
        } else {
            for _path in &self.paths {
                todo!();
            }
        }
        Ok(())
    }
}
