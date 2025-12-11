use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use oxvg_ast::{
    parse::roxmltree::parse_with_options,
    xmlwriter::{Indent, Options, Space},
};
use roxmltree::ParsingOptions;

use crate::{
    args::RunCommand,
    config::Config,
    walk::{Output, Walk},
};

#[derive(clap::Args, Debug)]
/// Runs [`Optimise`] with options and defaults specialised for formatting
pub struct Format {
    /// The target paths to optimise
    #[clap(value_parser)]
    pub paths: Vec<PathBuf>,
    /// Whether to write to the specified file or directory.
    /// Will use the input if flag is given without a value.
    /// Defaults to standard output.
    #[clap(long, short, num_args(0..=1))]
    pub output: Option<Vec<PathBuf>>,
    /// If the path is a directory, whether to walk through and optimise its subdirectories
    #[clap(long, short, default_value = "false")]
    pub recursive: bool,
    /// Search through hidden files and directories.
    ///
    /// A file or directory is considered hidden if its base name starts with a '.' or if the operating
    /// system provides a "hidden" file attribute.
    ///
    /// Ignored files will continue to be skipped and can be enabled with the `--no-ignore` flag.
    #[clap(long, short = '.', default_value = "false")]
    pub hidden: bool,
    /// When set, patterns defined in files such as `.gitigore` will be disregarded.
    ///
    /// Hidden files will continue to be skipped and can be enabled with the `--hidden` flag.
    #[clap(long, default_value = "false")]
    pub no_ignore: bool,
    /// Sets the approximate number of threads to use. A value of 0 (default) will automatically determine the appropriate number
    #[clap(long, short, default_value = "0")]
    pub threads: usize,
    /// When running without a config, sets the default preset to run with
    #[clap(long, short, default_value = "4")]
    pub pretty: Indent,
    /// Controls how the output handles whitespace.
    #[clap(long, short, default_value = "auto")]
    pub space: Space,
}

impl RunCommand for Format {
    async fn run(self, _config: Config) -> anyhow::Result<()> {
        let walk = Walk {
            paths: &self.paths,
            output: self.output.as_ref().and_then(|output| output.first()),
            recursive: self.recursive,
            hidden: self.hidden,
            no_ignore: self.no_ignore,
            threads: self.threads,
        };
        let error = Arc::new(AtomicBool::new(false));
        walk.run(|| {
            let error = Arc::clone(&error);
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
                    |dom, _| -> anyhow::Result<()> {
                        let output = Output {
                            options: format_options,
                            dom,
                            input: path,
                            destination: output,
                            input_size: source.len() as f64,
                        };
                        output.output()?;
                        Ok(())
                    },
                );
                error.store(true, Ordering::Relaxed);
                match result {
                    Err(err) => eprintln!("{err}"),
                    Ok(Err(err)) => eprintln!("{err}"),
                    Ok(Ok(())) => {}
                }
            })
        })?;
        if error.load(Ordering::Relaxed) {
            Err(anyhow::anyhow!("Failed to format document!"))
        } else {
            Ok(())
        }
    }
}
