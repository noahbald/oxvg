use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use oxvg_actions::Actor;
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
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
/// Runs a set of commands against a document.
///
/// # Examples
///
/// ```sh
///
/// cat example.svg | oxvg action -- -select "main" -delete > example.updated.svg
///
/// ```
pub struct Action {
    #[command(subcommand)]
    command: Option<ActionCommands>,
    #[command(flatten)]
    run: ActionRun,
}

#[derive(clap::Subcommand, Debug)]
pub enum ActionCommands {
    /// Runs a set of commands against a document.
    ///
    /// # Examples
    ///
    /// ```sh
    ///
    /// cat example.svg | oxvg action run -- -select "main" -delete > example.updated.svg
    ///
    /// ```
    Run(ActionRun),
    /// Prints out the set of commands.
    ///
    /// # Examples
    ///
    /// Prints out the spec for each action specified.
    ///
    /// ```sh
    ///
    /// # Prints spec for `-select` and `-delete`
    ///
    /// oxvg action list -- -select "main" -delete
    ///
    /// ```
    ///
    /// Prints out the spec for all possible actions.
    ///
    /// ```sh
    ///
    /// oxvg action list
    ///
    /// ```
    List(ActionList),
}

#[derive(clap::Args, Debug)]
pub struct ActionRun {
    #[clap(flatten)]
    /// Walk options
    walk: Walk,
    #[clap(long, short, default_value = "false")]
    /// Instead of outputting the modified document, it will output the state and information
    /// for the current selection of the modified document as a JSON string.
    pub derive_state: bool,
    #[arg(last = true)]
    /// The list of actions to apply to the document in the format `-<action> ("<arg>")+`.
    pub command_list: Vec<String>,
    /// When running without a config, sets the default preset to run with
    #[clap(long, short, default_value = "4")]
    pub pretty: Indent,
    /// Controls how the output handles whitespace.
    #[clap(long, short, default_value = "auto")]
    pub space: Space,
}

#[derive(clap::Args, Debug)]
#[command(ignore_errors(true))]
pub struct ActionList {
    #[arg(last = true)]
    /// The list of actions to apply to the document in the format `-<action> ("<arg>")+`.
    pub command_list: Vec<String>,
}

impl RunCommand for Action {
    async fn run(self, config: Config) -> anyhow::Result<()> {
        if let Some(subcommand) = self.command {
            subcommand.run(config).await
        } else {
            self.run.run(config).await
        }
    }
}

impl RunCommand for ActionCommands {
    async fn run(self, config: Config) -> anyhow::Result<()> {
        match self {
            Self::Run(args) => args.run(config).await,
            Self::List(args) => args.run(config).await,
        }
    }
}

impl RunCommand for ActionRun {
    async fn run(self, _: Config) -> anyhow::Result<()> {
        let actions = parse(self.command_list)?;
        let error = Arc::new(AtomicBool::new(false));
        self.walk.run(|| {
            let actions = actions.clone();
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
                    |dom, allocator| -> anyhow::Result<()> {
                        let mut actor =
                            Actor::new(dom, allocator).map_err(|err| anyhow::anyhow!("{err}"))?;
                        for action in actions.clone() {
                            actor
                                .dispatch(action.clone())
                                .map_err(|err| anyhow::anyhow!("{err}"))?;
                        }
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
        })
    }
}

impl RunCommand for ActionList {
    async fn run(self, _: Config) -> anyhow::Result<()> {
        let parts: HashSet<_> = self.command_list.into_iter().collect();

        if parts.is_empty() || parts.contains(FORGET) {
            println!("# Select\n");
            println!(include_str!(
                "../../../oxvg_actions/src/spec/state/forget.md"
            ));
        }
        if parts.is_empty() || parts.contains(SELECT) {
            println!("# Forget\n");
            println!(include_str!(
                "../../../oxvg_actions/src/spec/state/select.md"
            ));
        }
        Ok(())
    }
}

const FORGET: &str = "-forget";
const SELECT: &str = "-select";

fn parse(command_list: Vec<String>) -> anyhow::Result<Vec<oxvg_actions::Action<'static>>> {
    let mut actions = Vec::with_capacity(
        command_list
            .iter()
            .filter(|part| part.starts_with('-'))
            .count(),
    );
    let mut parts = command_list.into_iter().peekable();
    while let Some(action) = parts.next() {
        if !action.starts_with('-') {
            return Err(anyhow::anyhow!("Expected command name, found {action}"));
        }
        actions.push(match action.as_str() {
            FORGET => oxvg_actions::Action::Forget,
            SELECT => oxvg_actions::Action::Select(
                parts
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("`{action}` missing query"))?
                    .into(),
            ),
            _ => return Err(anyhow::anyhow!("Unknown action `{action}`")),
        });
    }
    Ok(actions)
}
