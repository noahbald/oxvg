use std::{
    ffi::OsStr,
    io::Write as _,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use oxvg_ast::parse::roxmltree::parse_with_options;
use oxvg_jsx::{
    config::{
        ExpandProps, ExportType, Icon, JSXRuntime, State, Template, TemplateString, VariablesAST,
    },
    error::{ConfigError, TemplateError},
    transform,
};
use oxvg_optimiser::Extends;

use roxmltree::ParsingOptions;
use swc_core::{
    common::{sync::Lrc, BytePos, SourceMap, DUMMY_SP},
    ecma::{
        ast::{
            ArrowExpr, BindingIdent, Constructor, Decl, EsVersion, Expr, ExprStmt, Function, Ident,
            IdentName, JSXElementChild, Module, ModuleDecl, ModuleItem, Param, ParamOrTsParamProp,
            Program, Stmt,
        },
        codegen::{text_writer::JsWriter, Emitter, Node as _},
        visit::{VisitMut, VisitMutWith},
    },
};
use swc_ecma_lexer::{Lexer, Parser, StringInput, Syntax};

use crate::{
    args::RunCommand,
    config::{self, Config},
    walk::Walk,
};

macro_rules! declare_template {
    ($template_ast:ident) => {
        $template_ast.clone().map(|t| {
            Template::<_, _, TemplateString<_>>::AST(
                move |w, ctxt, _state| {
                    let mut t_ = (*t).clone();
                    drop(t);
                    t_.visit_mut_with(&mut ApplyTemplate(ctxt));
                    let document = Program::Module(t_);

                    let cm = Lrc::<SourceMap>::default();
                    let mut emitter = Emitter {
                        cfg: swc_core::ecma::codegen::Config::default(),
                        cm: cm.clone(),
                        comments: None,
                        wr: JsWriter::new(cm, "\n", w, None),
                    };
                    document.emit_with(&mut emitter).map_err(|_| TemplateError)
                },
                PhantomData,
            )
        })
    };
}

/// Configures what changes are made when transforming an SVG document to JSX.
#[derive(clap::Args, Debug)]
pub struct JSXConfig {
    /// Whether to add `ref` attribute with `forwardRef`.
    #[clap(long)]
    pub r#ref: Option<bool>,
    /// Whether to add `title` and `titleId` prop to pass to `<title>` element.
    #[clap(long)]
    pub title_prop: Option<bool>,
    /// Whether to add `desc` and `descId` prop to pass to `<desc>` element.
    #[clap(long)]
    pub desc_prop: Option<bool>,
    /// Whether to pass `...props` to `<svg>` element.
    #[clap(long)]
    pub expand_props: Option<ExpandProps>,
    /// Whether to strip `width` and `height` attributes from `<svg>` element.
    #[clap(long)]
    pub no_dimensions: bool,
    /// Whether to set `width` and `height` to specified icon type.
    #[clap(long)]
    pub icon: Option<String>,
    /// Whether to use React-Native elements.
    #[clap(long)]
    pub native: Option<bool>,
    /// A set of attributes to apply to `<svg>` element. Values given as `"{...}"` will
    /// be parsed as JSX expressions instead of strings.
    ///
    /// A value of `--svg-props "fill=currentColor"` will add the attribute `fill="currentColor"`
    #[clap(long)]
    pub svg_props: Option<Vec<String>>,
    /// A set of values to replace the original values specified in the keys. Values given as `"{...}"` will
    /// be parsed as JSX expressions instead of strings.
    ///
    /// A value of `--replace-attr-values "#000=currentColor"` will replace all `#000` values with `"currentColor"`.
    #[clap(long)]
    pub replace_attr_values: Option<Vec<String>>,
    /// Whether to generate TypeScript types.
    #[clap(long)]
    pub typescript: Option<bool>,
    /// Whether to use `oxvg_optimiser` to optimise SVG document before processing it.
    ///
    /// Will use `optimise.jobs` from the config file when specified.
    #[clap(long)]
    pub no_oxvg: bool,
    /// Whether to use `React.memo` on the exported component.
    #[clap(long)]
    pub memo: Option<bool>,
    /// Whether to use `default` keyword when exporting component.
    #[clap(long)]
    pub export_type: Option<ExportType>,
    /// An alias for the export when using a named export-type.
    #[clap(long)]
    pub named_export: Option<String>,
    /// When given, controls where to import JSX runtime.
    #[clap(long)]
    pub jsx_runtime: Option<JSXRuntime>,
    /// Disable warnings
    #[clap(long)]
    pub no_warn: bool,
}

#[derive(clap::Args, Debug)]
/// Transforms an SVG document into JSX.
///
/// # Examples
///
/// ```sh
/// cat example.svg | oxvg jsx > Example.jsx
/// ```
pub struct JSX {
    #[clap(flatten)]
    /// Walk options
    pub walk: Walk,
    /// A path to the specified config.
    /// If no config is specified the current config will be printed instead.
    ///
    /// This job will use the optimisation options and jsx options by default. Options
    /// in the config can be overridden by using CLI flags instead.
    #[clap(long, short, num_args(0..=1))]
    pub config_file: Option<Vec<PathBuf>>,
    #[clap(long, short)]
    /// When running without a config, sets the default optimisation preset to run with
    pub extends: Option<Extends>,
    #[clap(flatten)]
    /// JSX config
    pub config: Option<JSXConfig>,
    /// When given, will parse a JS file and replace `$`-prefixed idents with the template
    /// variables.
    ///
    /// # Variables
    ///
    /// - `$imports`: A module declaration. Will only be replaced if the ident is in a root statement.
    /// - `$interfaces`: Interface declarations. Will only be replaced if the ident is in a root statement.
    /// - `$componentName`: The component name. Will replace any valid ident expression.
    /// - `$jsx`: The JSX root child node. Will replace any valid ident expression.
    /// - `$props`: The component parameters. Will replace if the ident is in a function parameter list.
    /// - `$exports`: A module declaration. Will only be replaced if the ident is in a root statement.
    ///
    /// # Example
    ///
    /// ```ts
    /// // Input
    /// $imports
    /// import PropTypes from 'prop-types';
    /// $interfaces
    ///
    /// function $componentName($props) {
    ///   return $jsx;
    /// }
    ///
    /// $componentName.propTypes = {
    ///   title: PropTypes.string,
    /// };
    ///
    /// $exports
    /// ```
    ///
    /// ```ts
    /// // Output
    /// import * as React from "react";
    /// import PropTypes from "prop-types";
    ///
    /// function SvgComponent(props) {
    ///   return <svg {...props}><g/></svg>;
    /// }
    ///
    /// SvgComponent.propTypes = {
    ///   title: PropTypes.string,
    /// };
    ///
    /// export default SvgComponent;
    /// ```
    #[clap(long)]
    pub template: Option<PathBuf>,
    /// Disable `index.js` file generation.
    #[clap(long)]
    pub no_index: bool,
}

impl TryInto<oxvg_jsx::Config> for JSXConfig {
    type Error = ConfigError;

    fn try_into(self) -> Result<oxvg_jsx::Config, ConfigError> {
        Ok(oxvg_jsx::Config {
            r#ref: self.r#ref,
            title_prop: self.title_prop,
            desc_prop: self.desc_prop,
            expand_props: self.expand_props,
            dimensions: Some(!self.no_dimensions),
            icon: self.icon.map(|i| match i.as_str() {
                "true" => Icon::Bool(true),
                "false" => Icon::Bool(false),
                s => match s.parse::<f64>() {
                    Ok(n) => Icon::Number(n),
                    Err(_) => Icon::String(i),
                },
            }),
            native: self.native,
            svg_props: self
                .svg_props
                .map(|v| {
                    v.into_iter()
                        .map(|s| {
                            s.split_once('=')
                                .ok_or_else(|| ConfigError::InvalidExpr(s.clone()))
                                .map(|(a, b)| (a.into(), b.into()))
                        })
                        .collect()
                })
                .transpose()?,
            replace_attr_values: self
                .replace_attr_values
                .map(|v| {
                    v.into_iter()
                        .map(|s| {
                            s.split_once('=')
                                .ok_or_else(|| ConfigError::InvalidExpr(s.clone()))
                                .map(|(a, b)| (a.into(), b.into()))
                        })
                        .collect()
                })
                .transpose()?,
            typescript: self.typescript,
            oxvg: Some(!self.no_oxvg),
            oxvg_config: None,
            memo: self.memo,
            export_type: self.export_type,
            named_export: self.named_export,
            jsx_runtime: self.jsx_runtime,
            jsx_runtime_import: None,
            warn: Some(!self.no_warn),
        })
    }
}

impl JSX {}

impl RunCommand for JSX {
    async fn run(self, config: Config) -> anyhow::Result<()> {
        let error = Arc::new(AtomicBool::new(false));
        let count = Arc::new(AtomicUsize::new(0));
        let jsx_config = resolve_config(self.config, &config)?;

        swc_core::common::GLOBALS.set(&swc_core::common::Globals::new(), || {
            let template_string = self.template.map(std::fs::read_to_string).transpose()?;
            let template_ast = parse_template(template_string)?;

            self.walk.run(|| {
                let error = Arc::clone(&error);
                let count = Arc::clone(&count);
                let jsx_config = jsx_config.clone();
                let template_ast = template_ast.clone();

                Box::new(move |source, path, output| {
                    let jsx_config = jsx_config.clone();
                    let template_ast = template_ast.clone();
                    let result = parse_with_options(
                        source,
                        ParsingOptions {
                            allow_dtd: true,
                            ..ParsingOptions::default()
                        },
                        #[allow(clippy::cast_precision_loss)]
                        |code, allocator| -> anyhow::Result<()> {
                            let mut output = output.or(path).cloned();
                            let typescript = jsx_config.typescript.unwrap_or(false);
                            let export_type = jsx_config.export_type.unwrap_or_default();
                            if let Some(output) = &mut output {
                                output.set_extension(if typescript { "tsx" } else { "jsx" });
                            }
                            let state = path
                                .and_then(|p| p.file_stem())
                                .map(|p| State {
                                    component_name: path_to_component_name(p),
                                })
                                .unwrap_or_default();

                            if let Some(output) = output {
                                if let Some(parent) = output.parent() {
                                    std::fs::create_dir_all(parent)?;
                                }
                                let component_name = state.component_name.clone();
                                let template = declare_template!(template_ast);
                                let file = std::fs::File::create(&output)?;
                                let mut buf = std::io::BufWriter::new(file);
                                transform(
                                    code,
                                    allocator,
                                    Some(jsx_config),
                                    Some(state),
                                    template,
                                    &mut buf,
                                )
                                .map_err(|err| anyhow::Error::msg(err.to_string()))?;
                                count.update(Ordering::Relaxed, Ordering::Relaxed, |n| n + 1);
                                append_to_index_js(
                                    &output,
                                    typescript,
                                    export_type,
                                    &component_name,
                                )
                            } else {
                                let template = declare_template!(template_ast);
                                transform(
                                    code,
                                    allocator,
                                    Some(jsx_config),
                                    Some(state),
                                    template,
                                    &mut std::io::stdout(),
                                )
                                .map_err(|err| anyhow::Error::msg(err.to_string()))
                            }
                        },
                    );
                    if matches!(result, Err(_) | Ok(Err(_))) {
                        error.store(true, Ordering::Relaxed);
                    }
                    match result {
                        Err(err) => eprintln!("{path:?}: {err}"),
                        Ok(Err(err)) => eprintln!("{err}"),
                        Ok(Ok(())) => {}
                    }
                })
            })
        })?;
        if error.load(Ordering::Relaxed) {
            Err(anyhow::anyhow!("Failed to transform all documents!"))
        } else {
            eprintln!("Created {} files.", count.load(Ordering::Relaxed));
            Ok(())
        }
    }
}

struct ApplyTemplate(VariablesAST);
impl VisitMut for ApplyTemplate {
    fn visit_mut_ident(&mut self, node: &mut Ident) {
        if node.sym == "$componentName" {
            *node = self.0.component_name.as_str().into();
        }
    }
    fn visit_mut_binding_ident(&mut self, node: &mut BindingIdent) {
        if node.sym == "$componentName" {
            *node = self.0.component_name.as_str().into();
        }
    }
    fn visit_mut_ident_name(&mut self, node: &mut IdentName) {
        if node.sym == "$componentName" {
            *node = self.0.component_name.as_str().into();
        }
    }

    fn visit_mut_expr(&mut self, node: &mut Expr) {
        node.visit_mut_children_with(self);

        if let Expr::Ident(ident) = node {
            if ident.sym == "$jsx" {
                match self.0.jsx.clone() {
                    JSXElementChild::JSXElement(e) => *node = Expr::JSXElement(e),
                    JSXElementChild::JSXFragment(e) => *node = Expr::JSXFragment(e),
                    _ => unreachable!(),
                }
            }
        }
    }

    fn visit_mut_constructor(&mut self, node: &mut Constructor) {
        node.visit_mut_children_with(self);

        let insertions: Vec<_> = node
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.as_param().map(|p| (i, &p.pat)))
            .filter_map(|(i, p)| p.as_ident().map(|p| (i, p)))
            .filter(|(_, p)| p.id.sym == "$props")
            .map(|(i, _)| i)
            .rev()
            .collect();
        for i in insertions {
            node.params.remove(i);
            for pat in self.0.props.iter().rev() {
                node.params.insert(
                    i,
                    ParamOrTsParamProp::Param(Param {
                        span: DUMMY_SP,
                        decorators: vec![],
                        pat: pat.clone(),
                    }),
                );
            }
        }
    }

    fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
        node.visit_mut_children_with(self);

        let insertions: Vec<_> = node
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.as_ident().map(|p| (i, p)))
            .filter(|(_, p)| p.id.sym == "$props")
            .map(|(i, _)| i)
            .rev()
            .collect();
        for i in insertions {
            node.params.remove(i);
            for pat in self.0.props.iter().rev() {
                node.params.insert(i, pat.clone());
            }
        }
    }

    fn visit_mut_function(&mut self, node: &mut Function) {
        node.visit_mut_children_with(self);

        let insertions: Vec<_> = node
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.pat.as_ident().map(|p| (i, p)))
            .filter(|(_, p)| p.id.sym == "$props")
            .map(|(i, _)| i)
            .rev()
            .collect();
        for i in insertions {
            node.params.remove(i);
            for pat in self.0.props.iter().rev() {
                node.params.insert(
                    i,
                    Param {
                        span: DUMMY_SP,
                        decorators: vec![],
                        pat: pat.clone(),
                    },
                );
            }
        }
    }

    fn visit_mut_module_items(&mut self, node: &mut Vec<ModuleItem>) {
        const IMPORTS: &str = "$imports";
        const INTERFACES: &str = "$interfaces";
        const EXPORTS: &str = "$exports";

        for node in node.iter_mut() {
            node.visit_mut_with(self);
        }

        let insertions: Vec<_> = node
            .iter()
            .enumerate()
            .filter_map(|(i, m)| {
                if let ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) = m {
                    if let Expr::Ident(ident) = &**expr {
                        match ident.sym.as_str() {
                            IMPORTS => Some((i, IMPORTS)),
                            INTERFACES => Some((i, INTERFACES)),
                            EXPORTS => Some((i, EXPORTS)),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .rev()
            .collect();
        for (insertion, name) in insertions {
            node.remove(insertion);
            match name {
                IMPORTS => {
                    for import in self.0.imports.iter().rev() {
                        node.insert(
                            insertion,
                            ModuleItem::ModuleDecl(ModuleDecl::Import(import.clone())),
                        );
                    }
                }
                INTERFACES => {
                    for interface in self.0.interfaces.iter().rev() {
                        node.insert(
                            insertion,
                            ModuleItem::Stmt(Stmt::Decl(Decl::TsInterface(Box::new(
                                interface.clone(),
                            )))),
                        );
                    }
                }
                EXPORTS => {
                    for export in self.0.exports.iter().rev() {
                        node.insert(insertion, export.clone());
                    }
                }
                _ => {}
            }
        }
    }
}

fn resolve_config(
    jsx_config: Option<JSXConfig>,
    config: &Config,
) -> anyhow::Result<oxvg_jsx::Config> {
    let mut jsx_config: oxvg_jsx::Config = jsx_config
        .map(TryInto::try_into)
        .transpose()?
        .unwrap_or_default();
    jsx_config.oxvg_config = config.optimise.as_ref().map(config::Optimise::resolve_jobs);
    Ok(jsx_config)
}

fn parse_template(template: Option<String>) -> anyhow::Result<Option<Arc<Module>>> {
    Ok(template
        .map(|t| {
            let lexer = Lexer::new(
                Syntax::default(),
                EsVersion::default(),
                StringInput::new(&t, BytePos(0), BytePos(t.len().max(1) as u32)),
                None,
            );
            let mut parser = Parser::new_from(lexer);
            parser
                .parse_module()
                .map_err(|err| anyhow::Error::msg(err.into_kind().msg()))
        })
        .transpose()?
        .map(Arc::new))
}

fn path_to_component_name(path: &OsStr) -> String {
    let mut next_capital = true;
    path.to_string_lossy()
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                if next_capital {
                    next_capital = false;
                    Some(c.to_ascii_uppercase())
                } else {
                    Some(c)
                }
            } else {
                next_capital = true;
                None
            }
        })
        .skip_while(char::is_ascii_digit)
        .collect()
}

fn append_to_index_js(
    output: &Path,
    typescript: bool,
    export_type: ExportType,
    component_name: &str,
) -> anyhow::Result<()> {
    if let (Some(parent), Some(file_stem)) = (output.parent(), output.file_stem()) {
        let index = parent.join(if typescript { "index.ts" } else { "index.js" });
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(index)?;
        let mut buf = std::io::BufWriter::new(file);
        match export_type {
            ExportType::Named => {
                writeln!(
                    buf,
                    r#"export {{ {component_name} }} from "./{}""#,
                    file_stem.to_string_lossy()
                )
            }
            ExportType::Default => {
                writeln!(
                    buf,
                    r#"export {{ default as {component_name} }} from "./{}""#,
                    file_stem.to_string_lossy()
                )
            }
        }?;
    }
    Ok(())
}
