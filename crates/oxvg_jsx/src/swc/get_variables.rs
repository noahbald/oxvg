//! A reimplementation of `@svgr/babel-plugin-transform-svg-component` in Rust.
use std::io::Write;

use itertools::Itertools as _;
use swc_core::{
    common::{sync::Lrc, SourceMap, SyntaxContext, DUMMY_SP},
    ecma::{
        ast::{
            ArrowExpr, AssignPatProp, BindingIdent, BlockStmtOrExpr, CallExpr, Callee, Decl,
            ExportDefaultExpr, ExportNamedSpecifier, ExportSpecifier, Expr, ExprOrSpread, Ident,
            ImportDecl, ImportDefaultSpecifier, ImportNamedSpecifier, ImportPhase, ImportSpecifier,
            ImportStarAsSpecifier, JSXElementChild, Module, ModuleDecl, ModuleExportName,
            ModuleItem, NamedExport, ObjectPat, ObjectPatProp, Pat, Program, RestPat, Stmt,
            TsEntityName, TsInterfaceBody, TsInterfaceDecl, TsIntersectionType, TsKeywordType,
            TsKeywordTypeKind, TsPropertySignature, TsType, TsTypeAnn, TsTypeElement,
            TsTypeParamInstantiation, TsTypeRef, TsUnionOrIntersectionType, VarDecl, VarDeclKind,
            VarDeclarator,
        },
        codegen::{text_writer::JsWriter, to_code, Emitter, Node},
    },
};

use crate::{
    config::{JsxRuntimeImport, TemplateContext, TemplateContextOptions, VariablesString},
    error::{ConfigError, TemplateError},
};

/// A non-serialized set of chunks, to be passed to the default template.
pub struct Variables {
    /// The name for this component as referenced by `exports`.
    pub component_name: String,
    /// The component's function parameters.
    pub props: Vec<Pat>,
    /// The TypeScript interfaces, as referenced by `prop`.
    pub interfaces: Vec<TsInterfaceDecl>,
    /// The imports, as referenced by other variables
    pub imports: Vec<ImportDecl>,
    /// The set of exports for the component module.
    pub exports: Vec<ModuleItem>,
    /// The JSX node returns by the component.
    pub jsx: JSXElementChild,
}

impl Variables {
    /// Builds the chunks of a JSX module using the given options.
    ///
    /// # Errors
    ///
    /// If there is an error parsing the config.
    #[allow(clippy::too_many_lines)]
    pub fn new(
        jsx: JSXElementChild,
        opts: &TemplateContextOptions,
    ) -> Result<Variables, ConfigError> {
        let mut interfaces = vec![];
        let mut props = vec![];
        let mut import_jsx_runtime_namespace = None;
        let mut import_jsx_runtime = None;
        let mut import_native = None;
        let mut import_jsx_runtime_default = None;
        let mut exports = vec![];
        let export_ident = opts.state.component_name.as_str();
        let props_ident: Ident = format!("{export_ident}Props").into();
        let mut export_ident: Ident = export_ident.into();

        let jsx_runtime_import = opts.jsx_runtime_import();
        get_jsx_runtime_import(
            jsx_runtime_import,
            opts,
            &mut import_jsx_runtime_namespace,
            &mut import_jsx_runtime,
            &mut import_jsx_runtime_default,
        )?;
        if opts.native() {
            let mut specifiers = Vec::with_capacity(opts.native_idents.len());
            if opts.native_idents.contains("Svg") {
                specifiers.push(ImportSpecifier::Default(ImportDefaultSpecifier {
                    span: DUMMY_SP,
                    local: "Svg".into(),
                }));
            }
            if opts.typescript() && opts.expand_props().is_some() {
                specifiers.push(ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: "SVGProps".into(),
                    imported: None,
                    is_type_only: false,
                }));
            }
            specifiers.extend(opts.native_idents.iter().filter(|i| **i != "Svg").map(|i| {
                ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: i.as_str().into(),
                    imported: None,
                    is_type_only: false,
                })
            }));
            if opts.title_prop() && !opts.native_idents.contains("Title") {
                specifiers.push(ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: "Title".into(),
                    imported: None,
                    is_type_only: false,
                }));
            }
            if opts.desc_prop() && !opts.native_idents.contains("Desc") {
                specifiers.push(ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: "Desc".into(),
                    imported: None,
                    is_type_only: false,
                }));
            }
            import_native = Some(ImportDecl {
                span: DUMMY_SP,
                specifiers,
                src: Box::new("react-native-svg".into()),
                type_only: false,
                with: None,
                phase: ImportPhase::Evaluation,
            });
        }

        if opts.title_prop() || opts.desc_prop() {
            let mut properties = ObjectPat {
                span: DUMMY_SP,
                props: vec![],
                optional: false,
                type_ann: None,
            };
            let mut property_signatures = vec![];
            let create_property = |name: &str| {
                ObjectPatProp::Assign(AssignPatProp {
                    span: DUMMY_SP,
                    key: BindingIdent {
                        id: name.into(),
                        type_ann: None,
                    },
                    value: None,
                })
            };
            let create_signature = |name: &str| {
                TsTypeElement::TsPropertySignature(TsPropertySignature {
                    span: DUMMY_SP,
                    readonly: false,
                    key: Box::new(Expr::Ident(name.into())),
                    computed: false,
                    optional: true,
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(
                            TsKeywordType {
                                span: DUMMY_SP,
                                kind: TsKeywordTypeKind::TsStringKeyword,
                            }
                            .into(),
                        ),
                    })),
                })
            };

            if opts.title_prop() {
                properties.props.push(create_property("title"));
                properties.props.push(create_property("titleId"));

                if opts.typescript() {
                    property_signatures.push(create_signature("title"));
                    property_signatures.push(create_signature("titleId"));
                }
            }

            if opts.desc_prop() {
                properties.props.push(create_property("desc"));
                properties.props.push(create_property("descId"));

                if opts.typescript() {
                    property_signatures.push(create_signature("desc"));
                    property_signatures.push(create_signature("descId"));
                }
            }

            if opts.typescript() {
                interfaces.push(TsInterfaceDecl {
                    span: DUMMY_SP,
                    id: props_ident.clone(),
                    declare: false,
                    type_params: None,
                    extends: vec![],
                    body: TsInterfaceBody {
                        span: DUMMY_SP,
                        body: property_signatures,
                    },
                });
                properties.type_ann = Some(Box::new(TsTypeAnn {
                    span: DUMMY_SP,
                    type_ann: if opts.expand_props().is_some() {
                        Box::new(TsType::TsUnionOrIntersectionType(
                            TsUnionOrIntersectionType::TsIntersectionType(TsIntersectionType {
                                span: DUMMY_SP,
                                types: vec![
                                    Box::new(TsType::TsTypeRef(TsTypeRef {
                                        span: DUMMY_SP,
                                        type_name: TsEntityName::Ident("SVGProps".into()),
                                        type_params: Some(Box::new(TsTypeParamInstantiation {
                                            span: DUMMY_SP,
                                            params: vec![Box::new(TsType::TsTypeRef(TsTypeRef {
                                                span: DUMMY_SP,
                                                type_name: TsEntityName::Ident(
                                                    "SVGSVGElement".into(),
                                                ),
                                                type_params: None,
                                            }))],
                                        })),
                                    })),
                                    Box::new(TsType::TsTypeRef(TsTypeRef {
                                        span: DUMMY_SP,
                                        type_name: TsEntityName::Ident(props_ident.clone()),
                                        type_params: None,
                                    })),
                                ],
                            }),
                        ))
                    } else {
                        Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(props_ident.clone()),
                            type_params: None,
                        }))
                    },
                }));
            }
            props.push(Pat::Object(properties));
        }

        if opts.expand_props().is_some() {
            let identifier = "props".into();
            if let Some(prop) = props.first_mut() {
                if let Pat::Object(object_pat) = prop {
                    object_pat.props.push(ObjectPatProp::Rest(RestPat {
                        span: DUMMY_SP,
                        dot3_token: DUMMY_SP,
                        arg: Box::new(Pat::Ident(BindingIdent {
                            id: identifier,
                            type_ann: None,
                        })),
                        type_ann: None,
                    }));
                } else {
                    debug_assert!(false, "premature non-object prop assigned");
                }
            } else {
                props.push(Pat::Ident(BindingIdent {
                    id: identifier,
                    type_ann: if opts.typescript() {
                        Some(Box::new(TsTypeAnn {
                            span: DUMMY_SP,
                            type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                                span: DUMMY_SP,
                                type_name: TsEntityName::Ident(props_ident),
                                type_params: None,
                            })),
                        }))
                    } else {
                        None
                    },
                }));
            }
        }

        if opts.r#ref() {
            if props.is_empty() {
                props.push(Pat::Ident(BindingIdent {
                    id: "_".into(),
                    type_ann: None,
                }));
            }
            let prop = "ref".into();
            props.push(Pat::Ident(BindingIdent {
                id: prop,
                type_ann: if opts.typescript() {
                    Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident("Ref".into()),
                            type_params: Some(Box::new(TsTypeParamInstantiation {
                                span: DUMMY_SP,
                                params: vec![Box::new(TsType::TsTypeRef(TsTypeRef {
                                    span: DUMMY_SP,
                                    type_name: TsEntityName::Ident("SVGSVGElement".into()),
                                    type_params: None,
                                }))],
                            })),
                        })),
                    }))
                } else {
                    None
                },
            }));
            let old_export_ident = export_ident.clone();
            export_ident = "ForwardRef".into();
            let forward_ref_f = "forwardRef".into();
            exports.push(VarDecl {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                kind: VarDeclKind::Const,
                declare: false,
                decls: vec![VarDeclarator {
                    span: DUMMY_SP,
                    name: Pat::Ident(BindingIdent {
                        id: export_ident.clone(),
                        type_ann: None,
                    }),
                    init: Some(Box::new(Expr::Call(CallExpr {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        callee: Callee::Expr(Box::new(Expr::Ident(forward_ref_f))),
                        args: vec![ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Ident(old_export_ident)),
                        }],
                        type_args: None,
                    }))),
                    definite: false,
                }],
            });
        }

        if opts.memo() {
            let old_export_ident = export_ident.clone();
            export_ident = "Memo".into();
            let forward_ref_f = "memo".into();
            exports.push(VarDecl {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                kind: VarDeclKind::Const,
                declare: false,
                decls: vec![VarDeclarator {
                    span: DUMMY_SP,
                    name: Pat::Ident(BindingIdent {
                        id: export_ident.clone(),
                        type_ann: None,
                    }),
                    init: Some(Box::new(Expr::Call(CallExpr {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        callee: Callee::Expr(Box::new(Expr::Ident(forward_ref_f))),
                        args: vec![ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Ident(old_export_ident)),
                        }],
                        type_args: None,
                    }))),
                    definite: false,
                }],
            });
        }

        let mut imports = vec![];
        if let Some(import) = import_jsx_runtime_namespace {
            imports.push(import);
        }
        if let Some(import) = import_native {
            imports.push(import);
        }
        if let Some(import) = import_jsx_runtime {
            imports.push(import);
        }
        if let Some(import) = import_jsx_runtime_default {
            imports.push(import);
        }

        let exports_iter = exports.into_iter();
        let mut exports = Vec::with_capacity(exports_iter.len() + 1);
        exports.extend(
            exports_iter
                .into_iter()
                .map(|i| ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(i))))),
        );
        exports.push(ModuleItem::ModuleDecl(
            if matches!(opts.export_type(), crate::config::ExportType::Default) {
                ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                    span: DUMMY_SP,
                    expr: Box::new(Expr::Ident(export_ident)),
                })
            } else {
                if let Some(n) = &opts.named_export {
                    if Ident::verify_symbol(n).is_err() {
                        return Err(ConfigError::InvalidIdent(n.clone()));
                    }
                }
                ModuleDecl::ExportNamed(NamedExport {
                    span: DUMMY_SP,
                    specifiers: vec![ExportSpecifier::Named(ExportNamedSpecifier {
                        span: DUMMY_SP,
                        orig: ModuleExportName::Ident(export_ident),
                        exported: opts
                            .named_export
                            .as_ref()
                            .map(|n| ModuleExportName::Ident(n.as_str().into())),
                        is_type_only: false,
                    })],
                    src: None,
                    type_only: false,
                    with: None,
                })
            },
        ));
        Ok(Self {
            component_name: opts.state.component_name.clone(),
            props,
            interfaces,
            imports,
            exports,
            jsx,
        })
    }
}

impl From<Variables> for VariablesString {
    fn from(value: Variables) -> Self {
        Self {
            component_name: value.component_name,
            props: value.props.iter().map(to_code).join(", "),
            interfaces: value
                .interfaces
                .into_iter()
                .map(|i| to_code(&Stmt::Decl(Decl::TsInterface(Box::new(i)))))
                .join("\n"),
            imports: value
                .imports
                .into_iter()
                .map(|i| to_code(&ModuleDecl::Import(i)))
                .join("\n"),
            exports: value.exports.iter().map(to_code).join("\n"),
            jsx: to_code(&value.jsx),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn get_jsx_runtime_import(
    jsx_runtime_import: JsxRuntimeImport,
    opts: &TemplateContextOptions,
    import_jsx_runtime_namespace: &mut Option<ImportDecl>,
    import_jsx_runtime: &mut Option<ImportDecl>,
    import_jsx_runtime_default: &mut Option<ImportDecl>,
) -> Result<(), ConfigError> {
    let mut specifiers = vec![];
    if let Some(named_specifiers) = jsx_runtime_import.specifiers {
        for specifier in named_specifiers {
            if Ident::verify_symbol(&specifier).is_ok() {
                specifiers.push(ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: specifier.into(),
                    imported: None,
                    is_type_only: false,
                }));
            } else {
                return Err(ConfigError::InvalidIdent(specifier));
            }
        }
    }

    let namespace_source = jsx_runtime_import
        .source_namespace
        .as_deref()
        .unwrap_or(jsx_runtime_import.source.as_str());
    let mut namespace_specifiers = vec![];
    if let Some(specifier) = jsx_runtime_import.namespace {
        if Ident::verify_symbol(&specifier).is_ok() {
            namespace_specifiers.push(ImportSpecifier::Namespace(ImportStarAsSpecifier {
                span: DUMMY_SP,
                local: specifier.into(),
            }));
        } else {
            return Err(ConfigError::InvalidIdent(specifier));
        }
    }
    if let Some(specifier) = jsx_runtime_import.default_specifier {
        if Ident::verify_symbol(&specifier).is_ok() {
            *import_jsx_runtime_default = Some(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                    span: DUMMY_SP,
                    local: specifier.into(),
                })],
                src: Box::new(namespace_source.into()),
                type_only: false,
                with: None,
                phase: ImportPhase::Evaluation,
            });
        } else {
            return Err(ConfigError::InvalidIdent(specifier));
        }
    }

    if opts.r#ref() {
        let specifier = ImportSpecifier::Named(ImportNamedSpecifier {
            span: DUMMY_SP,
            local: "forwardRef".into(),
            imported: None,
            is_type_only: false,
        });
        if jsx_runtime_import.source_namespace.is_some() {
            namespace_specifiers.push(specifier);
        } else {
            specifiers.push(specifier);
        }
    }
    if opts.memo() {
        let specifier = ImportSpecifier::Named(ImportNamedSpecifier {
            span: DUMMY_SP,
            local: "memo".into(),
            imported: None,
            is_type_only: false,
        });
        if jsx_runtime_import.source_namespace.is_some() {
            namespace_specifiers.push(specifier);
        } else {
            specifiers.push(specifier);
        }
    }
    if opts.expand_props().is_some() && opts.typescript() && !opts.native() {
        let specifier = ImportSpecifier::Named(ImportNamedSpecifier {
            span: DUMMY_SP,
            local: "SVGProps".into(),
            imported: None,
            is_type_only: false,
        });
        if jsx_runtime_import.source_namespace.is_some() {
            namespace_specifiers.push(specifier);
        } else {
            specifiers.push(specifier);
        }
    }
    if !namespace_specifiers.is_empty() {
        *import_jsx_runtime_namespace = Some(ImportDecl {
            span: DUMMY_SP,
            specifiers: namespace_specifiers,
            src: Box::new(namespace_source.into()),
            type_only: false,
            with: None,
            phase: ImportPhase::Evaluation,
        });
    }
    if !specifiers.is_empty() {
        *import_jsx_runtime = Some(ImportDecl {
            span: DUMMY_SP,
            specifiers,
            src: Box::new(jsx_runtime_import.source.into()),
            type_only: false,
            with: None,
            phase: ImportPhase::Evaluation,
        });
    }
    Ok(())
}

/// Applies the default template to the given writer.
///
/// # Errors
///
/// If there's an error building the document.
pub fn default_template<W: Write>(
    w: &mut W,
    variables: Variables,
    _context: TemplateContext,
) -> Result<(), TemplateError> {
    let mut body: Vec<_> = variables
        .imports
        .into_iter()
        .map(|import| ModuleItem::ModuleDecl(ModuleDecl::Import(import)))
        .collect();
    body.extend(
        variables
            .interfaces
            .into_iter()
            .map(|interface| ModuleItem::Stmt(Stmt::Decl(Decl::TsInterface(Box::new(interface))))),
    );
    body.push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: variables.component_name.as_str().into(),
                type_ann: None,
            }),
            init: Some(Box::new(Expr::Arrow(ArrowExpr {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                params: variables.props,
                is_async: false,
                is_generator: false,
                type_params: None,
                return_type: None,
                body: Box::new(BlockStmtOrExpr::Expr(Box::new(match variables.jsx {
                    JSXElementChild::JSXFragment(fragment) => Expr::JSXFragment(fragment),
                    JSXElementChild::JSXElement(element) => Expr::JSXElement(element),
                    _ => unreachable!(),
                }))),
            }))),
            definite: false,
        }],
    })))));
    body.extend(variables.exports);

    let document = Program::Module(Module {
        span: DUMMY_SP,
        body,
        shebang: None,
    });

    let cm = Lrc::<SourceMap>::default();
    let mut emitter = Emitter {
        cfg: swc_core::ecma::codegen::Config::default(),
        cm: cm.clone(),
        comments: None,
        wr: JsWriter::new(cm, "\n", w, None),
    };
    document.emit_with(&mut emitter).map_err(|_| TemplateError)
}
