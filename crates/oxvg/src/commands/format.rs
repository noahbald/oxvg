use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
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
/// Runs [`crate::commands::Optimise`] with options and defaults specialised for formatting
pub struct Format {
    #[clap(flatten)]
    /// Walk options
    pub walk: Walk,
    /// When running without a config, sets the default preset to run with
    #[clap(long, short, default_value = "4")]
    pub pretty: Indent,
    /// Controls how the output handles whitespace.
    #[clap(long, short, default_value = "auto")]
    pub space: Space,
}

impl RunCommand for Format {
    async fn run(self, _config: Config) -> anyhow::Result<()> {
        let error = Arc::new(AtomicBool::new(false));
        self.walk.run(|| {
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
                    #[allow(clippy::cast_precision_loss)]
                    |dom, _| -> anyhow::Result<()> {
                        let output = Output {
                            options: format_options,
                            dom,
                            input: path,
                            destination: output,
                            input_bytes: source.len() as f64,
                        };
                        output.output()?;
                        Ok(())
                    },
                );
                if matches!(result, Err(_) | Ok(Err(_))) {
                    error.store(true, Ordering::Relaxed);
                }
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
