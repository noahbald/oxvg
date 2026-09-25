//! A reimplementation of `@svgr/babel-plugin-transform-svg-component` in Rust.
use std::io::Write;

use itertools::Itertools as _;
use oxc_allocator::{ArenaVec, Box, FromIn, GetAllocator, Vec};
use oxc_ast::{
    ast::{
        Argument, BindingIdentifier, BindingPattern, BindingProperty, BindingRestElement,
        ExportSpecifier, Expression, FormalParameter, FormalParameterKind, FormalParameters,
        FunctionBody, Ident, IdentifierName, ImportDeclaration, ImportDeclarationSpecifier,
        ImportOrExportKind, JSXChild, ModuleExportName, ObjectPattern, Program, PropertyKey,
        SourceType, Span, Statement, Str, StringLiteral, TSInterfaceBody, TSInterfaceDeclaration,
        TSSignature, TSType, TSTypeAnnotation, TSTypeName, TSTypeParameterInstantiation,
        VariableDeclarationKind, VariableDeclarator,
    },
    builder::GetAstBuilder,
};
use oxc_codegen::{Codegen, Gen};
use oxc_syntax::identifier::is_identifier_name;

use crate::{
    config::{
        ExportType, JsxRuntimeImport, TemplateContext, TemplateContextOptions, VariablesString,
    },
    error::{ConfigError, TemplateError},
};

/// A non-serialized set of chunks, to be passed to the default template.
pub struct Variables<'alloc> {
    /// The name for this component as referenced by `exports`.
    pub component_name: String,
    /// The component's function parameters.
    pub props: Box<'alloc, FormalParameters<'alloc>>,
    /// The TypeScript interfaces, as referenced by `prop`.
    pub interfaces: Vec<'alloc, Box<'alloc, TSInterfaceDeclaration<'alloc>>>,
    /// The imports, as referenced by other variables
    pub imports: Vec<'alloc, Box<'alloc, ImportDeclaration<'alloc>>>,
    /// The set of exports for the component module.
    pub exports: Vec<'alloc, Statement<'alloc>>,
    /// The JSX node returns by the component.
    pub jsx: JSXChild<'alloc>,
}

impl<'alloc> Variables<'alloc> {
    /// Builds the chunks of a JSX module using the given options.
    ///
    /// # Errors
    ///
    /// If there is an error parsing the config.
    #[allow(clippy::too_many_lines)]
    pub fn new<B: GetAstBuilder<'alloc>>(
        jsx: JSXChild<'alloc>,
        opts: &TemplateContextOptions,
        builder: &B,
    ) -> Result<Variables<'alloc>, ConfigError> {
        let builder = builder.builder();
        let mut interfaces = Vec::new_in(builder);
        let mut props = FormalParameters::boxed(
            Span::default(),
            FormalParameterKind::ArrowFormalParameters,
            Vec::new_in(builder),
            Option::<Box<'alloc, _>>::None,
            builder,
        );
        let mut import_jsx_runtime_namespace = None;
        let mut import_jsx_runtime = None;
        let mut import_native = None;
        let mut import_jsx_runtime_default = None;
        let mut exports = Vec::new_in(builder);
        let export_ident = opts.state.component_name.as_str();
        let props_ident = Str::from_in(format!("{export_ident}Props"), builder.allocator());
        let mut export_ident = Ident::from_str_in(export_ident, builder);

        let jsx_runtime_import = opts.jsx_runtime_import();
        get_jsx_runtime_import(
            jsx_runtime_import,
            opts,
            &mut import_jsx_runtime_namespace,
            &mut import_jsx_runtime,
            &mut import_jsx_runtime_default,
            builder,
        )?;
        if opts.native() {
            let mut specifiers = Vec::with_capacity_in(opts.native_idents.len(), builder);
            if opts.native_idents.contains("Svg") {
                specifiers.push(ImportDeclarationSpecifier::new_import_default_specifier(
                    Span::default(),
                    BindingIdentifier::new(Span::default(), "Svg", builder),
                    builder,
                ));
            }
            if opts.typescript() && opts.expand_props().is_some() {
                specifiers.push(ImportDeclarationSpecifier::new_import_specifier(
                    Span::default(),
                    ModuleExportName::new_identifier_name(Span::default(), "SVGProps", builder),
                    BindingIdentifier::new(Span::default(), "SVGProps", builder),
                    ImportOrExportKind::Value,
                    builder,
                ))
            }
            specifiers.extend(opts.native_idents.iter().filter(|i| **i != "Svg").map(|i| {
                ImportDeclarationSpecifier::new_import_specifier(
                    Span::default(),
                    ModuleExportName::new_identifier_name(
                        Span::default(),
                        Str::from_str_in(i, builder),
                        builder,
                    ),
                    BindingIdentifier::new(Span::default(), Str::from_str_in(i, builder), builder),
                    ImportOrExportKind::Value,
                    builder,
                )
            }));
            if opts.title_prop() && !opts.native_idents.contains("Title") {
                specifiers.push(ImportDeclarationSpecifier::new_import_specifier(
                    Span::default(),
                    ModuleExportName::new_identifier_name(Span::default(), "Title", builder),
                    BindingIdentifier::new(Span::default(), "Title", builder),
                    ImportOrExportKind::Value,
                    builder,
                ))
            }
            if opts.desc_prop() && !opts.native_idents.contains("Desc") {
                specifiers.push(ImportDeclarationSpecifier::new_import_specifier(
                    Span::default(),
                    ModuleExportName::new_identifier_name(Span::default(), "Desc", builder),
                    BindingIdentifier::new(Span::default(), "Desc", builder),
                    ImportOrExportKind::Value,
                    builder,
                ))
            }
            import_native = Some(ImportDeclaration::boxed(
                Span::default(),
                Some(specifiers),
                StringLiteral::new(
                    Span::default(),
                    Str::new_const("react-native-svg"),
                    None,
                    builder,
                ),
                None,
                Option::<Box<'alloc, _>>::None,
                ImportOrExportKind::Value,
                builder,
            ))
        }

        if opts.title_prop() || opts.desc_prop() {
            let mut properties = ObjectPattern::boxed(
                Span::default(),
                Vec::new_in(builder),
                Option::<Box<'alloc, _>>::None,
                builder,
            );
            let mut property_signatures = Vec::new_in(builder);
            let create_property = |name: &'static str| {
                BindingProperty::new(
                    Span::default(),
                    PropertyKey::StaticIdentifier(IdentifierName::boxed(
                        Span::default(),
                        name,
                        builder,
                    )),
                    BindingPattern::new_binding_identifier(
                        Span::default(),
                        Str::new_const(name),
                        builder,
                    ),
                    true,
                    false,
                    builder,
                )
            };
            let create_signature = |name: &'static str| {
                TSSignature::new_ts_property_signature(
                    Span::default(),
                    false,
                    true,
                    false,
                    PropertyKey::new_static_identifier(
                        Span::default(),
                        Str::new_const(name),
                        builder,
                    ),
                    Some(TSTypeAnnotation::new(
                        Span::default(),
                        TSType::new_ts_string_keyword(Span::default(), builder),
                        builder,
                    )),
                    builder,
                )
            };

            if opts.title_prop() {
                properties.properties.push(create_property("title"));
                properties.properties.push(create_property("titleId"));

                if opts.typescript() {
                    property_signatures.push(create_signature("title"));
                    property_signatures.push(create_signature("titleId"));
                }
            }

            if opts.desc_prop() {
                properties.properties.push(create_property("desc"));
                properties.properties.push(create_property("descId"));

                if opts.typescript() {
                    property_signatures.push(create_signature("desc"));
                    property_signatures.push(create_signature("descId"));
                }
            }

            let mut formal_parameter = FormalParameter::new_plain(
                Span::default(),
                BindingPattern::ObjectPattern(properties),
                builder,
            );
            if opts.typescript() {
                interfaces.push(TSInterfaceDeclaration::boxed(
                    Span::default(),
                    BindingIdentifier::new(Span::default(), props_ident, builder),
                    Option::<Box<'alloc, _>>::None,
                    Vec::new_in(builder),
                    TSInterfaceBody::boxed(Span::default(), property_signatures, builder),
                    false,
                    builder,
                ));
                formal_parameter.type_annotation = Some(TSTypeAnnotation::boxed(
                    Span::default(),
                    if opts.expand_props().is_some() {
                        TSType::new_ts_intersection_type(
                            Span::default(),
                            Vec::from_iter_in(
                                [
                                    TSType::new_ts_type_reference(
                                        Span::default(),
                                        TSTypeName::new_identifier_reference(
                                            Span::default(),
                                            Str::new_const("SVGProps"),
                                            builder,
                                        ),
                                        Some(TSTypeParameterInstantiation::boxed(
                                            Span::default(),
                                            Vec::from_value_in(
                                                TSType::new_ts_type_reference(
                                                    Span::default(),
                                                    TSTypeName::new_identifier_reference(
                                                        Span::default(),
                                                        Str::new_const("SVGSVGElement"),
                                                        builder,
                                                    ),
                                                    Option::<Box<'alloc, _>>::None,
                                                    builder,
                                                ),
                                                builder,
                                            ),
                                            builder,
                                        )),
                                        builder,
                                    ),
                                    TSType::new_ts_type_reference(
                                        Span::default(),
                                        TSTypeName::new_identifier_reference(
                                            Span::default(),
                                            props_ident.clone(),
                                            builder,
                                        ),
                                        Option::<Box<'alloc, _>>::None,
                                        builder,
                                    ),
                                ]
                                .into_iter(),
                                builder,
                            ),
                            builder,
                        )
                    } else {
                        TSType::new_ts_type_reference(
                            Span::default(),
                            TSTypeName::new_identifier_reference(
                                Span::default(),
                                props_ident.clone(),
                                builder,
                            ),
                            Option::<Box<'alloc, _>>::None,
                            builder,
                        )
                    },
                    builder,
                ));
            }
            props.items.push(formal_parameter)
        }

        if opts.expand_props().is_some() {
            let identifier = Str::new_const("props");
            if let Some(prop) = props.items.first_mut() {
                if let BindingPattern::ObjectPattern(object_pat) = &mut prop.pattern {
                    object_pat.rest = Some(BindingRestElement::boxed(
                        Span::default(),
                        BindingPattern::new_binding_identifier(
                            Span::default(),
                            identifier,
                            builder,
                        ),
                        builder,
                    ))
                } else {
                    debug_assert!(false, "premature non-object prop assigned");
                }
            } else {
                props.items.push(FormalParameter::new(
                    Span::default(),
                    Vec::new_in(builder),
                    BindingPattern::new_binding_identifier(Span::default(), identifier, builder),
                    if opts.typescript() {
                        Some(TSTypeAnnotation::boxed(
                            Span::default(),
                            TSType::new_ts_type_reference(
                                Span::default(),
                                TSTypeName::new_identifier_reference(
                                    Span::default(),
                                    props_ident,
                                    builder,
                                ),
                                Option::<Box<'alloc, _>>::None,
                                builder,
                            ),
                            builder,
                        ))
                    } else {
                        None
                    },
                    Option::<Box<'alloc, _>>::None,
                    false,
                    None,
                    false,
                    false,
                    builder,
                ))
            }
        }

        if opts.r#ref() {
            if props.is_empty() {
                props.items.push(FormalParameter::new_plain(
                    Span::default(),
                    BindingPattern::new_binding_identifier(
                        Span::default(),
                        Str::new_const("_"),
                        builder,
                    ),
                    builder,
                ))
            }
            let prop = Str::new_const("ref");
            props.items.push(FormalParameter::new(
                Span::default(),
                Vec::new_in(builder),
                BindingPattern::new_binding_identifier(Span::default(), prop, builder),
                if opts.typescript() {
                    Some(TSTypeAnnotation::boxed(
                        Span::default(),
                        TSType::new_ts_type_reference(
                            Span::default(),
                            TSTypeName::new_identifier_reference(
                                Span::default(),
                                Str::new_const("SVGSVGElement"),
                                builder,
                            ),
                            Option::<Box<'alloc, _>>::None,
                            builder,
                        ),
                        builder,
                    ))
                } else {
                    None
                },
                Option::<Box<'alloc, _>>::None,
                false,
                None,
                false,
                false,
                builder,
            ));
            let old_export_ident = export_ident.clone();
            export_ident = Str::new_const("ForwardRef").into();
            let forward_ref_f = Str::new_const("forwardRef");
            exports.push(Statement::new_variable_declaration(
                Span::default(),
                VariableDeclarationKind::Const,
                Vec::from_value_in(
                    VariableDeclarator::new(
                        Span::default(),
                        VariableDeclarationKind::Const,
                        BindingPattern::new_binding_identifier(
                            Span::default(),
                            export_ident,
                            builder,
                        ),
                        Option::<Box<'alloc, _>>::None,
                        Some(Expression::new_call_expression(
                            Span::default(),
                            Expression::new_identifier(Span::default(), forward_ref_f, builder),
                            Option::<Box<'alloc, _>>::None,
                            Vec::from_value_in(
                                Argument::new_identifier(
                                    Span::default(),
                                    old_export_ident,
                                    builder,
                                ),
                                builder,
                            ),
                            false,
                            builder,
                        )),
                        false,
                        builder,
                    ),
                    builder,
                ),
                false,
                builder,
            ))
        }

        if opts.memo() {
            let old_export_ident = export_ident.clone();
            export_ident = Str::new_const("Memo").into();
            let forward_ref_f = Str::new_const("memo");
            exports.push(Statement::new_variable_declaration(
                Span::default(),
                VariableDeclarationKind::Const,
                Vec::from_value_in(
                    VariableDeclarator::new(
                        Span::default(),
                        VariableDeclarationKind::Const,
                        BindingPattern::new_binding_identifier(
                            Span::default(),
                            export_ident,
                            builder,
                        ),
                        Option::<Box<'alloc, _>>::None,
                        Some(Expression::new_call_expression(
                            Span::default(),
                            Expression::new_identifier(Span::default(), forward_ref_f, builder),
                            Option::<Box<'alloc, _>>::None,
                            Vec::from_value_in(
                                Argument::new_identifier(
                                    Span::default(),
                                    old_export_ident,
                                    builder,
                                ),
                                builder,
                            ),
                            false,
                            builder,
                        )),
                        false,
                        builder,
                    ),
                    builder,
                ),
                false,
                builder,
            ))
        }

        let mut imports = Vec::new_in(builder);
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

        exports.push(if matches!(opts.export_type(), ExportType::Default) {
            Statement::new_export_default_declaration(
                Span::default(),
                oxc_ast::ast::ExportDefaultDeclarationKind::new_identifier(
                    Span::default(),
                    export_ident,
                    builder,
                ),
                builder,
            )
        } else {
            if let Some(n) = &opts.named_export
                && !is_identifier_name(n)
            {
                return Err(ConfigError::InvalidIdent(n.clone()));
            }
            Statement::new_export_named_declaration(
                Span::default(),
                None,
                Vec::from_value_in(
                    ExportSpecifier::new(
                        Span::default(),
                        ModuleExportName::new_identifier_reference(
                            Span::default(),
                            export_ident,
                            builder,
                        ),
                        opts.named_export
                            .as_ref()
                            .map(|n| {
                                ModuleExportName::new_identifier_reference(
                                    Span::default(),
                                    Str::from_in(n, builder.allocator()),
                                    builder,
                                )
                            })
                            .unwrap_or_else(|| {
                                ModuleExportName::new_identifier_reference(
                                    Span::default(),
                                    export_ident,
                                    builder,
                                )
                            }),
                        ImportOrExportKind::Value,
                        builder,
                    ),
                    builder,
                ),
                None,
                ImportOrExportKind::Value,
                Option::<Box<'alloc, _>>::None,
                builder,
            )
        });
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

impl<'input> From<Variables<'input>> for VariablesString {
    fn from(value: Variables<'input>) -> Self {
        let ctx = oxc_codegen::Context::empty().with_typescript();
        Self {
            component_name: value.component_name,
            props: value
                .props
                .items
                .iter()
                .map(|prop| {
                    let mut codegen = Codegen::new();
                    prop.print(&mut codegen, ctx);
                    codegen.into_source_text()
                })
                .join(", "),
            interfaces: value
                .interfaces
                .into_iter()
                .map(Statement::TSInterfaceDeclaration)
                .map(|interface| {
                    let mut codegen = Codegen::new();
                    interface.print(&mut codegen, ctx);
                    codegen.into_source_text()
                })
                .join("\n"),
            imports: value
                .imports
                .into_iter()
                .map(Statement::ImportDeclaration)
                .map(|import| {
                    let mut codegen = Codegen::new();
                    import.print(&mut codegen, ctx);
                    codegen.into_source_text()
                })
                .join("\n"),
            exports: value
                .exports
                .into_iter()
                .map(|export| {
                    let mut codegen = Codegen::new();
                    export.print(&mut codegen, ctx);
                    codegen.into_source_text()
                })
                .join("\n"),
            jsx: {
                let mut codegen = Codegen::new();
                value.jsx.print(&mut codegen, ctx);
                codegen.into_source_text()
            },
        }
    }
}

#[allow(clippy::too_many_lines)]
fn get_jsx_runtime_import<'alloc, B: GetAstBuilder<'alloc>>(
    jsx_runtime_import: JsxRuntimeImport,
    opts: &TemplateContextOptions,
    import_jsx_runtime_namespace: &mut Option<Box<'alloc, ImportDeclaration<'alloc>>>,
    import_jsx_runtime: &mut Option<Box<'alloc, ImportDeclaration<'alloc>>>,
    import_jsx_runtime_default: &mut Option<Box<'alloc, ImportDeclaration<'alloc>>>,
    builder: &B,
) -> Result<(), ConfigError> {
    let builder = builder.builder();
    let mut specifiers = Vec::new_in(builder);
    if let Some(named_specifiers) = jsx_runtime_import.specifiers {
        for specifier in named_specifiers {
            if is_identifier_name(&specifier) {
                let specifier = Str::from_in(specifier, builder.allocator());
                specifiers.push(ImportDeclarationSpecifier::new_import_specifier(
                    Span::default(),
                    ModuleExportName::new_identifier_name(Span::default(), specifier, builder),
                    BindingIdentifier::new(Span::default(), specifier, builder),
                    ImportOrExportKind::Value,
                    builder,
                ))
            } else {
                return Err(ConfigError::InvalidIdent(specifier));
            }
        }
    }

    let namespace_source = Str::from_str_in(
        jsx_runtime_import
            .source_namespace
            .as_deref()
            .unwrap_or(jsx_runtime_import.source.as_str()),
        builder,
    );
    let mut namespace_specifiers = Vec::new_in(builder);
    if let Some(specifier) = jsx_runtime_import.namespace {
        if is_identifier_name(&specifier) {
            namespace_specifiers.push(ImportDeclarationSpecifier::new_import_namespace_specifier(
                Span::default(),
                BindingIdentifier::new(
                    Span::default(),
                    Str::from_in(specifier, builder.allocator()),
                    builder,
                ),
                builder,
            ))
        } else {
            return Err(ConfigError::InvalidIdent(specifier));
        }
    }
    if let Some(specifier) = jsx_runtime_import.default_specifier {
        if is_identifier_name(&specifier) {
            *import_jsx_runtime_default = Some(ImportDeclaration::boxed(
                Span::default(),
                Some(Vec::from_value_in(
                    ImportDeclarationSpecifier::new_import_default_specifier(
                        Span::default(),
                        BindingIdentifier::new(
                            Span::default(),
                            Str::from_in(specifier, builder.allocator()),
                            builder,
                        ),
                        builder,
                    ),
                    builder,
                )),
                StringLiteral::new(Span::default(), namespace_source, None, builder),
                None,
                Option::<Box<'alloc, _>>::None,
                ImportOrExportKind::Value,
                builder,
            ))
        } else {
            return Err(ConfigError::InvalidIdent(specifier));
        }
    }

    if opts.r#ref() {
        let specifier = Str::new_const("forwardRef");
        let specifier = ImportDeclarationSpecifier::new_import_specifier(
            Span::default(),
            ModuleExportName::new_identifier_name(Span::default(), specifier, builder),
            BindingIdentifier::new(Span::default(), specifier, builder),
            ImportOrExportKind::Value,
            builder,
        );
        if jsx_runtime_import.source_namespace.is_some() {
            namespace_specifiers.push(specifier);
        } else {
            specifiers.push(specifier);
        }
    }
    if opts.memo() {
        let specifier = Str::new_const("memo");
        let specifier = ImportDeclarationSpecifier::new_import_specifier(
            Span::default(),
            ModuleExportName::new_identifier_name(Span::default(), specifier, builder),
            BindingIdentifier::new(Span::default(), specifier, builder),
            ImportOrExportKind::Value,
            builder,
        );
        if jsx_runtime_import.source_namespace.is_some() {
            namespace_specifiers.push(specifier);
        } else {
            specifiers.push(specifier);
        }
    }
    if opts.expand_props().is_some() && opts.typescript() && !opts.native() {
        let specifier = Str::new_const("SVGProps");
        let specifier = ImportDeclarationSpecifier::new_import_specifier(
            Span::default(),
            ModuleExportName::new_identifier_name(Span::default(), specifier, builder),
            BindingIdentifier::new(Span::default(), specifier, builder),
            ImportOrExportKind::Value,
            builder,
        );
        if jsx_runtime_import.source_namespace.is_some() {
            namespace_specifiers.push(specifier);
        } else {
            specifiers.push(specifier);
        }
    }
    if !namespace_specifiers.is_empty() {
        *import_jsx_runtime_namespace = Some(ImportDeclaration::boxed(
            Span::default(),
            Some(namespace_specifiers),
            StringLiteral::new(Span::default(), namespace_source, None, builder),
            None,
            Option::<Box<'alloc, _>>::None,
            ImportOrExportKind::Value,
            builder,
        ))
    }
    if !specifiers.is_empty() {
        *import_jsx_runtime = Some(ImportDeclaration::boxed(
            Span::default(),
            Some(specifiers),
            StringLiteral::new(
                Span::default(),
                Str::from_str_in(&jsx_runtime_import.source, builder),
                None,
                builder,
            ),
            None,
            Option::<Box<'alloc, _>>::None,
            ImportOrExportKind::Value,
            builder,
        ))
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
    variables: Variables<'_>,
    context: TemplateContext,
) -> Result<(), TemplateError> {
    let allocator = oxc_allocator::Allocator::new();
    let builder = oxc_ast::builder::AstBuilder::new(&allocator);
    let mut body = Vec::from_iter_in(
        variables
            .imports
            .into_iter()
            .map(Statement::ImportDeclaration),
        &builder,
    );
    body.extend(
        variables
            .interfaces
            .into_iter()
            .map(Statement::TSInterfaceDeclaration),
    );
    body.push(Statement::new_variable_declaration(
        Span::default(),
        VariableDeclarationKind::Const,
        Vec::from_value_in(
            VariableDeclarator::new(
                Span::default(),
                VariableDeclarationKind::Const,
                BindingPattern::new_binding_identifier(
                    Span::default(),
                    Str::from_in(variables.component_name, builder.allocator()),
                    &builder,
                ),
                Option::<Box<_>>::None,
                Some(Expression::new_arrow_function_expression(
                    Span::default(),
                    true,
                    false,
                    Option::<Box<_>>::None,
                    variables.props,
                    Option::<Box<_>>::None,
                    FunctionBody::boxed(
                        Span::default(),
                        Vec::new_in(&builder),
                        Vec::from_value_in(
                            Statement::new_expression_statement(
                                Span::default(),
                                match variables.jsx {
                                    JSXChild::Fragment(fragment) => {
                                        Expression::JSXFragment(fragment)
                                    }
                                    JSXChild::Element(element) => Expression::JSXElement(element),
                                    _ => unreachable!(),
                                },
                                &builder,
                            ),
                            &builder,
                        ),
                        &builder,
                    ),
                    &builder,
                )),
                false,
                &builder,
            ),
            &builder,
        ),
        false,
        &builder,
    ));
    body.extend(variables.exports);

    let document = Program::new(
        Span::default(),
        if context.options.typescript() {
            SourceType::tsx()
        } else {
            SourceType::jsx()
        },
        "",
        ArenaVec::new_in(&builder),
        None,
        ArenaVec::new_in(&builder),
        body,
        &builder,
    );

    let result = Codegen::new().build(&document);
    w.write_fmt(format_args!("{}", result.code))
        .map_err(|_| TemplateError)
}
