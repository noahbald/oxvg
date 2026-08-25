use std::{
    collections::HashSet,
    future,
    iter::Peekable,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use oxvg_actions::Actor;
use oxvg_ast::{
    parse::roxmltree::parse_with_options,
    xmlwriter::{Indent, Options, Space},
};
use oxvg_collections::atom::Atom;
use oxvg_parse::Parse as _;
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
    fn run(self, _: Config) -> impl Future<Output = anyhow::Result<()>> + Send {
        let actions = match parse(self.command_list) {
            Ok(actions) => actions,
            Err(err) => return future::ready(Err(err)),
        };
        let error = Arc::new(AtomicBool::new(false));
        future::ready(self.walk.run(|| {
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
                            quiet: self.walk.quiet,
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
        }))
    }
}

impl RunCommand for ActionList {
    #[allow(clippy::too_many_lines)]
    fn run(self, _: Config) -> impl Future<Output = anyhow::Result<()>> + Send {
        let parts: HashSet<_> = self.command_list.into_iter().collect();

        if parts.is_empty() || parts.contains(ATTR) {
            println!("# Attribute\n");
            println!(include_str!("../spec/manipulate/attr.md"));
        }
        if parts.is_empty() || parts.contains(CLASS) {
            println!("# Class\n");
            println!(include_str!("../spec/manipulate/class.md"));
        }
        if parts.is_empty() || parts.contains(PATH_INTERSECT) {
            println!("# Path Intersect\n");
            println!(include_str!("../spec/manipulate/path_intersect.md"));
        }
        if parts.is_empty() || parts.contains(PATH_UNION) {
            println!("# Path Union\n");
            println!(include_str!("../spec/manipulate/path_union.md"));
        }
        if parts.is_empty() || parts.contains(PATH_SUBTRACT) {
            println!("# Path Subtract\n");
            println!(include_str!("../spec/manipulate/path_subtract.md"));
        }
        if parts.is_empty() || parts.contains(PATH_XOR) {
            println!("# Path Xor\n");
            println!(include_str!("../spec/manipulate/path_xor.md"));
        }
        if parts.is_empty() || parts.contains(STYLE) {
            println!("# Style\n");
            println!(include_str!("../spec/manipulate/style.md"));
        }
        if parts.is_empty() || parts.contains(MATRIX) {
            println!("# Matrix\n");
            println!(include_str!("../spec/manipulate/matrix.md"));
        }
        if parts.is_empty() || parts.contains(TRANSLATE) {
            println!("# Translate\n");
            println!(include_str!("../spec/manipulate/translate.md"));
        }
        if parts.is_empty() || parts.contains(SCALE) {
            println!("# Scale\n");
            println!(include_str!("../spec/manipulate/scale.md"));
        }
        if parts.is_empty() || parts.contains(ROTATE) {
            println!("# Rotate\n");
            println!(include_str!("../spec/manipulate/rotate.md"));
        }
        if parts.is_empty() || parts.contains(SKEW_X) {
            println!("# Skew X\n");
            println!(include_str!("../spec/manipulate/skewX.md"));
        }
        if parts.is_empty() || parts.contains(SKEW_Y) {
            println!("# Skew Y\n");
            println!(include_str!("../spec/manipulate/skewY.md"));
        }
        if parts.is_empty() || parts.contains(INSERT) || parts.contains(CREATE_ELEMENT) {
            println!("# Insert\n");
            println!(include_str!("../spec/structure/insert.md"));
        }
        if parts.is_empty() || parts.contains(INSERT_NS) || parts.contains(CREATE_ELEMENT_NS) {
            println!("# Insert NS\n");
            println!(include_str!("../spec/structure/insert_ns.md"));
        }
        if parts.is_empty() || parts.contains(DUPLICATE) {
            println!("# Duplicate\n");
            println!(include_str!("../spec/structure/duplicate.md"));
        }
        if parts.is_empty() || parts.contains(WRAP) {
            println!("# Wrap\n");
            println!(include_str!("../spec/structure/wrap.md"));
        }
        if parts.is_empty() || parts.contains(CLONE) {
            println!("# Clone\n");
            println!(include_str!("../spec/structure/clone.md"));
        }
        if parts.is_empty() || parts.contains(ANCHOR_LINK) {
            println!("# Anchor Link\n");
            println!(include_str!("../spec/structure/anchor_link.md"));
        }
        if parts.is_empty() || parts.contains(GROUP) {
            println!("# Group\n");
            println!(include_str!("../spec/structure/group.md"));
        }
        if parts.is_empty() || parts.contains(DELETE) {
            println!("# Delete\n");
            println!(include_str!("../spec/structure/delete.md"));
        }
        if parts.is_empty() || parts.contains(FORGET) {
            println!("# Forget\n");
            println!(include_str!("../spec/state/forget.md"));
        }
        if parts.is_empty() || parts.contains(SELECT) {
            println!("# Select\n");
            println!(include_str!("../spec/state/select.md"));
        }
        if parts.is_empty() || parts.contains(SELECT_MORE) {
            println!("# Select More\n");
            println!(include_str!("../spec/state/select-more.md"));
        }
        if parts.is_empty() || parts.contains(DESELECT) {
            println!("# Deselect\n");
            println!(include_str!("../spec/state/deselect.md"));
        }
        future::ready(Ok(()))
    }
}

const ATTR: &str = "-attr";
const CLASS: &str = "-class";
const PATH_INTERSECT: &str = "-path-intersect";
const PATH_UNION: &str = "-path-union";
const PATH_SUBTRACT: &str = "-path-subtract";
const PATH_XOR: &str = "-path-xor";
const STYLE: &str = "-style";
const MATRIX: &str = "-matrix";
const TRANSLATE: &str = "-translate";
const SCALE: &str = "-scale";
const ROTATE: &str = "-rotate";
const SKEW_X: &str = "-skewX";
const SKEW_Y: &str = "-skewY";
const INSERT: &str = "-insert";
const CREATE_ELEMENT: &str = "-create-element";
const INSERT_NS: &str = "-insert-ns";
const CREATE_ELEMENT_NS: &str = "-create-element-ns";
const DUPLICATE: &str = "-duplicate";
const WRAP: &str = "-wrap";
const CLONE: &str = "-clone";
const ANCHOR_LINK: &str = "-anchor-link";
const GROUP: &str = "-group";
const DELETE: &str = "-delete";
const FORGET: &str = "-forget";
const SELECT: &str = "-select";
const SELECT_MORE: &str = "-select-more";
const DESELECT: &str = "-deselect";

fn parse(command_list: Vec<String>) -> anyhow::Result<Vec<oxvg_actions::Action<'static>>> {
    let mut actions = Vec::with_capacity(
        command_list
            .iter()
            .filter(|part| part.starts_with('-'))
            .count(),
    );
    let mut parts = command_list.into_iter().peekable();
    while let Some(action) = parts.next() {
        let get_part = |parts: &mut Peekable<std::vec::IntoIter<String>>| {
            parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("`{action}` missing query"))
                .map(Atom::from)
        };
        let get_part_f32 =
            |parts: &mut Peekable<std::vec::IntoIter<String>>| -> anyhow::Result<f32> {
                f32::parse_string(get_part(parts)?.as_str()).map_err(|err| anyhow::anyhow!("{err}"))
            };
        let get_part_f32_peek = |parts: &mut Peekable<std::vec::IntoIter<String>>| {
            let n = {
                let part = parts.peek()?;
                if let Ok(n) = f32::parse_string(part) {
                    n
                } else {
                    return None;
                }
            };
            parts.next();
            Some(n)
        };
        if !action.starts_with('-') {
            return Err(anyhow::anyhow!("Expected command name, found {action}"));
        }
        actions.push(match action.as_str() {
            ATTR => oxvg_actions::Action::Attr {
                name: get_part(&mut parts)?,
                value: get_part(&mut parts)?,
            },
            CLASS => oxvg_actions::Action::Class(get_part(&mut parts)?),
            PATH_INTERSECT => oxvg_actions::Action::PathIntersect,
            PATH_UNION => oxvg_actions::Action::PathUnion,
            PATH_SUBTRACT => oxvg_actions::Action::PathSubtract,
            PATH_XOR => oxvg_actions::Action::PathXor,
            STYLE => oxvg_actions::Action::Style {
                property: get_part(&mut parts)?,
                value: get_part(&mut parts)?,
            },
            MATRIX => oxvg_actions::Action::Matrix(
                get_part_f32(&mut parts)?,
                get_part_f32(&mut parts)?,
                get_part_f32(&mut parts)?,
                get_part_f32(&mut parts)?,
                get_part_f32(&mut parts)?,
                get_part_f32(&mut parts)?,
            ),
            TRANSLATE => oxvg_actions::Action::Translate(
                get_part_f32(&mut parts)?,
                get_part_f32_peek(&mut parts),
            ),
            SCALE => oxvg_actions::Action::Scale(
                get_part_f32(&mut parts)?,
                get_part_f32_peek(&mut parts),
            ),
            ROTATE => oxvg_actions::Action::Rotate(
                get_part_f32(&mut parts)?,
                if let Some(x) = get_part_f32_peek(&mut parts) {
                    Some((x, get_part_f32(&mut parts)?))
                } else {
                    None
                },
            ),
            SKEW_X => oxvg_actions::Action::SkewX(get_part_f32(&mut parts)?),
            SKEW_Y => oxvg_actions::Action::SkewY(get_part_f32(&mut parts)?),
            INSERT | CREATE_ELEMENT => oxvg_actions::Action::Insert(get_part(&mut parts)?),
            INSERT_NS | CREATE_ELEMENT_NS => {
                oxvg_actions::Action::InsertNS(get_part(&mut parts)?, get_part(&mut parts)?)
            }
            DUPLICATE => oxvg_actions::Action::Duplicate,
            WRAP => oxvg_actions::Action::Wrap(get_part(&mut parts)?),
            CLONE => oxvg_actions::Action::Clone,
            ANCHOR_LINK => oxvg_actions::Action::AnchorLink(get_part(&mut parts)?),
            GROUP => oxvg_actions::Action::Group,
            DELETE => oxvg_actions::Action::Delete,
            FORGET => oxvg_actions::Action::Forget,
            SELECT => oxvg_actions::Action::Select(get_part(&mut parts)?),
            SELECT_MORE => oxvg_actions::Action::SelectMore(get_part(&mut parts)?),
            DESELECT => oxvg_actions::Action::Deselect,
            _ => return Err(anyhow::anyhow!("Unknown action `{action}`")),
        });
    }
    Ok(actions)
}
