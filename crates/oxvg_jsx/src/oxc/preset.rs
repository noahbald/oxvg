//! A reimplementation of `@svgr/babel-preset` and it's `@svgr` dependencies in Rust.
use std::collections::HashSet;

use oxc_allocator::{Box, CloneIn, GetAllocator, Vec};
use oxc_ast::{
    ast::{
        BinaryOperator, Expression, JSXAttribute, JSXAttributeItem, JSXAttributeName,
        JSXAttributeValue, JSXChild, JSXClosingElement, JSXElement, JSXElementName, JSXExpression,
        JSXExpressionContainer, JSXIdentifier, JSXOpeningElement, LogicalOperator, NumberBase,
        SourceType, Span, Str,
    },
    builder::GetAstBuilder,
};
use oxc_parser::Parser;

use crate::{
    BuildError, Config,
    config::{ExpandProps, Icon},
    error::{ConfigError, Error},
};

/// Mutates the given root JSX element based on the given config.
///
/// # Errors
///
/// If there is an error building the document or parsing the config.
#[allow(clippy::too_many_lines)]
pub fn preset<'alloc, 'err, S: std::hash::BuildHasher, B: GetAstBuilder<'alloc>>(
    jsx: &mut JSXChild<'alloc>,
    config: &Config,
    native_idents: &mut HashSet<&'static str, S>,
    builder: &B,
) -> Result<(), Error<'err>> {
    let builder = builder.builder();
    if config.native() {
        convert_tree_to_react_native(jsx, native_idents, builder);
    }
    let ident_svg = if config.native() { "Svg" } else { "svg" };
    let svg_element = get_first_named_jsx_element(jsx, ident_svg)
        .ok_or(BuildError::MissingSVGElement)
        .map_err(Error::BuildError)?;

    if let Some(svg_props) = &config.svg_props {
        for (key, value) in svg_props {
            let attr = parse_templatable_attr(value, builder).map_err(Error::ConfigError)?;
            set_attribute(
                svg_element,
                Str::from_str_in(key, builder.builder()),
                attr,
                true,
                builder,
            );
        }
    }

    if config.r#ref() {
        set_attribute(
            svg_element,
            Str::new_const("ref"),
            JSXAttributeValue::new_expression_container(
                Span::default(),
                JSXExpression::new_identifier(Span::default(), Str::new_const("ref"), builder),
                builder,
            ),
            true,
            builder,
        );
    }

    if config.title_prop() {
        config.title_prop();
        set_attribute(
            svg_element,
            Str::new_const("aria-labelledby"),
            JSXAttributeValue::new_expression_container(
                Span::default(),
                JSXExpression::new_identifier(Span::default(), Str::new_const("titleId"), builder),
                builder,
            ),
            true,
            builder,
        );
    }

    if config.desc_prop() {
        set_attribute(
            svg_element,
            Str::new_const("aria-describedby"),
            JSXAttributeValue::new_expression_container(
                Span::default(),
                JSXExpression::new_identifier(Span::default(), Str::new_const("descId"), builder),
                builder,
            ),
            true,
            builder,
        );
    }

    if !config.dimensions() {
        svg_element.opening_element.attributes.retain(|attr| {
            if let JSXAttributeItem::Attribute(attr) = attr
                && let JSXAttribute { name: attr, .. } = &**attr
            {
                !matches!(attr.get_identifier().name.as_str(), "width" | "height")
            } else {
                true
            }
        });
    }

    match &config.icon {
        Some(Icon::Bool(true)) => {
            let attr = if config.native() {
                JSXAttributeValue::new_expression_container(
                    Span::default(),
                    JSXExpression::new_numeric_literal(
                        Span::default(),
                        24.0,
                        None,
                        NumberBase::Decimal,
                        builder,
                    ),
                    builder,
                )
            } else {
                JSXAttributeValue::new_string_literal(
                    Span::default(),
                    Str::new_const("1em"),
                    None,
                    builder,
                )
            };
            set_attribute(
                svg_element,
                Str::new_const("width"),
                attr.clone_in(builder.allocator()),
                true,
                builder,
            );
            set_attribute(svg_element, Str::new_const("height"), attr, true, builder);
        }
        Some(Icon::String(str)) => {
            let attr = JSXAttributeValue::new_string_literal(
                Span::default(),
                Str::from_str_in(str, builder),
                None,
                builder,
            );
            set_attribute(
                svg_element,
                Str::new_const("width"),
                attr.clone_in(builder.allocator()),
                true,
                builder,
            );
            set_attribute(svg_element, Str::new_const("height"), attr, true, builder);
        }
        Some(Icon::Number(value)) => {
            let attr = JSXAttributeValue::new_expression_container(
                Span::default(),
                JSXExpression::new_numeric_literal(
                    Span::default(),
                    *value,
                    None,
                    NumberBase::Decimal,
                    builder,
                ),
                builder,
            );
            set_attribute(
                svg_element,
                Str::new_const("width"),
                attr.clone_in(builder.allocator()),
                true,
                builder,
            );
            set_attribute(svg_element, Str::new_const("height"), attr, true, builder);
        }
        Some(Icon::Bool(false)) | None => {}
    }

    // WARN: No pushing props after this!
    //       e.g. set_attribute_recursive(..., true)
    let element = JSXAttributeItem::new_spread_attribute(
        Span::default(),
        Expression::new_identifier(Span::default(), Str::new_const("props"), builder),
        builder,
    );
    match config.expand_props() {
        ExpandProps::Start => svg_element.opening_element.attributes.insert(0, element),
        ExpandProps::End => svg_element.opening_element.attributes.push(element),
        ExpandProps::None => {}
    }

    if let Some(replace_attr_values) = &config.replace_attr_values {
        for (key, value) in replace_attr_values {
            let attr = parse_templatable_attr(value, builder).map_err(Error::ConfigError)?;
            set_value_recursive(jsx, key, &attr, builder);
        }
    }

    if config.desc_prop() {
        let ident_desc = if config.native() { "Desc" } else { "desc" };
        replace_element_conditionally(jsx, ident_desc, "descId", "desc", ident_svg, builder)
            .map_err(Error::BuildError)?;
    }

    if config.title_prop() {
        let ident_title = if config.native() { "Title" } else { "title" };
        replace_element_conditionally(jsx, ident_title, "titleId", "title", ident_svg, builder)
            .map_err(Error::BuildError)?;
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn replace_element_conditionally<'alloc, B: GetAstBuilder<'alloc>>(
    jsx: &mut JSXChild<'alloc>,
    tag: &'static str,
    id: &'static str,
    ident_child: &'static str,
    ident_root: &str,
    builder: &B,
) -> Result<(), BuildError<'static>> {
    let builder = builder.builder();
    let existing = get_first_named_jsx_element_child(jsx, tag);
    let mut existing_id = None;
    let existing = if let Some(existing) = existing {
        let mut original_title = std::mem::replace(
            existing,
            JSXChild::new_expression_container(
                Span::default(),
                JSXExpression::new_empty_expression(Span::default(), builder),
                builder,
            ),
        );
        if let JSXChild::Element(element) = &mut original_title
            && let Some(attr) = element
                .opening_element
                .attributes
                .iter_mut()
                .filter_map(|a| {
                    if let JSXAttributeItem::Attribute(a) = a {
                        Some(a)
                    } else {
                        None
                    }
                })
                .find(|a| a.name.as_identifier().is_some_and(|a| a.name == "id"))
        {
            match &attr.value {
                Some(JSXAttributeValue::StringLiteral(s)) => {
                    attr.value = Some(JSXAttributeValue::ExpressionContainer(
                        JSXExpressionContainer::boxed(
                            Span::default(),
                            JSXExpression::new_logical_expression(
                                Span::default(),
                                Expression::new_identifier(
                                    Span::default(),
                                    Str::new_const(id),
                                    builder,
                                ),
                                LogicalOperator::Or,
                                Expression::new_string_literal(
                                    Span::default(),
                                    s.value.clone_in(builder.allocator()),
                                    None,
                                    builder,
                                ),
                                builder,
                            ),
                            builder,
                        ),
                    ));
                    existing_id = attr.value.clone_in(builder.allocator());
                }
                Some(JSXAttributeValue::ExpressionContainer(expr)) => {
                    attr.value = Some(JSXAttributeValue::new_expression_container(
                        Span::default(),
                        JSXExpression::new_logical_expression(
                            Span::default(),
                            Expression::new_identifier(
                                Span::default(),
                                Str::new_const(id),
                                builder,
                            ),
                            LogicalOperator::Or,
                            expr.expression
                                .clone_in(builder.allocator())
                                .into_expression(),
                            builder,
                        ),
                        builder,
                    ));
                    existing_id = attr.value.clone_in(builder.allocator());
                }
                _ => {
                    attr.value = Some(JSXAttributeValue::new_string_literal(
                        Span::default(),
                        Str::new_const(id),
                        None,
                        builder,
                    ))
                }
            }
        }
        Some((existing, original_title))
    } else {
        None
    };
    let inserting_element = JSXElement::boxed(
        Span::default(),
        JSXOpeningElement::new(
            Span::default(),
            JSXElementName::new_identifier(Span::default(), Str::new_const(tag), builder),
            Option::<Box<'alloc, _>>::None,
            Vec::from_value_in(
                JSXAttributeItem::new_attribute(
                    Span::default(),
                    JSXAttributeName::new_identifier(
                        Span::default(),
                        Str::new_const("id"),
                        builder,
                    ),
                    Some(existing_id.unwrap_or_else(|| {
                        JSXAttributeValue::new_expression_container(
                            Span::default(),
                            JSXExpression::new_identifier(
                                Span::default(),
                                Str::new_const(id),
                                builder,
                            ),
                            builder,
                        )
                    })),
                    builder,
                ),
                builder,
            ),
            builder,
        ),
        Vec::from_value_in(
            JSXChild::new_expression_container(
                Span::default(),
                JSXExpression::new_identifier(
                    Span::default(),
                    Str::new_const(ident_child),
                    builder,
                ),
                builder,
            ),
            builder,
        ),
        Some(JSXClosingElement::boxed(
            Span::default(),
            JSXElementName::new_identifier(Span::default(), tag, builder),
            builder,
        )),
        builder,
    );
    if let Some((existing, original_title)) = existing {
        if let JSXChild::Element(inner) = original_title {
            *existing = JSXChild::new_expression_container(
                Span::default(),
                JSXExpression::new_conditional_expression(
                    Span::default(),
                    Expression::new_binary_expression(
                        Span::default(),
                        Expression::new_identifier(
                            Span::default(),
                            Str::new_const(ident_child),
                            builder,
                        ),
                        BinaryOperator::StrictEquality,
                        Expression::new_identifier(
                            Span::default(),
                            Str::new_const("undefined"),
                            builder,
                        ),
                        builder,
                    ),
                    Expression::JSXElement(inner),
                    Expression::JSXElement(inserting_element),
                    builder,
                ),
                builder,
            );
        } else {
            *existing = original_title;
        }
    } else {
        drop(existing);
        let svg_element =
            get_first_named_jsx_element(jsx, ident_root).ok_or(BuildError::MissingSVGElement)?;
        svg_element.children.insert(
            0,
            JSXChild::new_expression_container(
                Span::default(),
                JSXExpression::new_conditional_expression(
                    Span::default(),
                    Expression::new_identifier(
                        Span::default(),
                        Str::new_const(ident_child),
                        builder,
                    ),
                    Expression::JSXElement(inserting_element),
                    Expression::new_null_literal(Span::default(), builder),
                    builder,
                ),
                builder,
            ),
        );
        svg_element.closing_element = Some(JSXClosingElement::boxed(
            Span::default(),
            svg_element
                .opening_element
                .name
                .clone_in(builder.allocator()),
            builder,
        ));
    }
    Ok(())
}

fn get_first_named_jsx_element_child<'a, 'alloc>(
    jsx: &'a mut JSXChild<'alloc>,
    name: &str,
) -> Option<&'a mut JSXChild<'alloc>> {
    let matched = match &*jsx {
        JSXChild::Element(element) => matches!(
            &element.opening_element.name.get_identifier_name(),
            Some(ident) if ident == name,
        ),
        _ => false,
    };
    if matched {
        return Some(jsx);
    }
    match jsx {
        JSXChild::Element(element) => {
            for child in &mut element.children {
                if let Some(child) = get_first_named_jsx_element_child(child, name) {
                    return Some(child);
                }
            }
            None
        }
        JSXChild::Fragment(element) => {
            for child in &mut element.children {
                if let Some(child) = get_first_named_jsx_element_child(child, name) {
                    return Some(child);
                }
            }
            None
        }
        _ => None,
    }
}

fn get_first_named_jsx_element<'a, 'alloc>(
    jsx: &'a mut JSXChild<'alloc>,
    name: &str,
) -> Option<&'a mut Box<'alloc, JSXElement<'alloc>>> {
    match jsx {
        JSXChild::Element(element) => match &element.opening_element.name.get_identifier_name() {
            Some(ident) if ident == name => Some(element),
            _ => {
                for child in &mut element.children {
                    if let Some(child) = get_first_named_jsx_element(child, name) {
                        return Some(child);
                    }
                }
                None
            }
        },
        JSXChild::Fragment(element) => {
            for child in &mut element.children {
                if let Some(child) = get_first_named_jsx_element(child, name) {
                    return Some(child);
                }
            }
            None
        }
        _ => None,
    }
}

fn parse_templatable_attr<'alloc, B: GetAstBuilder<'alloc>>(
    template: &str,
    builder: &B,
) -> Result<JSXAttributeValue<'alloc>, ConfigError> {
    if template.starts_with('{') && template.ends_with('}') {
        let input = &template[1..template.len() - 1];
        let parser = Parser::new(builder.builder().allocator(), input, SourceType::default());
        let expr = parser
            .parse_expression()
            .map_err(|err| {
                err.errors()
                    .next()
                    .map(|err| err.to_string())
                    .unwrap_or_else(|| "Unknown error".into())
            })
            .map_err(ConfigError::InvalidExpr)?;
        Ok(JSXAttributeValue::new_expression_container(
            Span::default(),
            expr.as_jsx_expression()
                .clone_in(builder.builder().allocator()),
            builder,
        ))
    } else {
        Ok(JSXAttributeValue::new_string_literal(
            Span::default(),
            Str::from_str_in(template, builder.builder()),
            None,
            builder,
        ))
    }
}

fn set_value_recursive<'alloc, B: GetAstBuilder<'alloc>>(
    root: &mut JSXChild<'alloc>,
    old_value: &str,
    new_value: &JSXAttributeValue<'alloc>,
    builder: &B,
) {
    match root {
        JSXChild::Element(element) => {
            for attr in &mut element.opening_element.attributes {
                if let JSXAttributeItem::Attribute(attr) = attr
                    && let Some(JSXAttributeValue::StringLiteral(str)) = &attr.value
                    && str.value.as_str() == old_value
                {
                    attr.value = Some(new_value.clone_in(builder.builder().allocator()));
                }
            }
            for child in &mut element.children {
                set_value_recursive(child, old_value, new_value, builder);
            }
        }
        JSXChild::Fragment(fragment) => {
            for child in &mut fragment.children {
                set_value_recursive(child, old_value, new_value, builder);
            }
        }
        _ => {}
    }
}

fn set_attribute<'alloc, B: GetAstBuilder<'alloc>>(
    element: &mut Box<JSXElement<'alloc>>,
    name: Str<'alloc>,
    new_value: JSXAttributeValue<'alloc>,
    allow_push: bool,
    builder: &B,
) {
    for attr in &mut element.opening_element.attributes {
        if let JSXAttributeItem::Attribute(attr) = attr
            && attr.name.as_identifier().is_some_and(|n| n.name == name)
        {
            attr.value = Some(new_value);
            return;
        }
    }
    if allow_push {
        element
            .opening_element
            .attributes
            .push(JSXAttributeItem::new_attribute(
                Span::default(),
                JSXAttributeName::new_identifier(Span::default(), name, builder),
                Some(new_value),
                builder,
            ))
    }
}

fn convert_tree_to_react_native<'alloc, S: std::hash::BuildHasher, B: GetAstBuilder<'alloc>>(
    jsx: &mut JSXChild<'alloc>,
    native_idents: &mut HashSet<&'static str, S>,
    builder: &B,
) -> bool {
    match jsx {
        JSXChild::Element(element) => match &mut element.opening_element.name {
            JSXElementName::Identifier(identifier) => {
                let new_name = match identifier.name.as_str() {
                    "svg" => "Svg",
                    "circle" => "Circle",
                    "clipPath" => "ClipPath",
                    "ellipse" => "Ellipse",
                    "g" => "G",
                    "linearGradient" => "LinearGradient",
                    "radialGradient" => "RadialGradient",
                    "line" => "Line",
                    "path" => "Path",
                    "pattern" => "Pattern",
                    "polygon" => "Polygon",
                    "polyline" => "Polyline",
                    "rect" => "Rect",
                    "symbol" => "Symbol",
                    "text" => "Text",
                    "textPath" => "TextPath",
                    "tspan" => "TSpan",
                    "use" => "Use",
                    "defs" => "Defs",
                    "stop" => "Stop",
                    "mask" => "Mask",
                    "image" => "Image",
                    "foreignObject" => "ForeignObject",
                    _ => return false,
                };
                native_idents.insert(new_name);
                *identifier =
                    JSXIdentifier::boxed(Span::default(), Str::new_const(new_name), builder);
                if let Some(closing_element) = &mut element.closing_element {
                    closing_element.name = JSXElementName::Identifier(JSXIdentifier::boxed(
                        Span::default(),
                        Str::new_const(new_name),
                        builder,
                    ));
                }
                element
                    .children
                    .retain_mut(|e| convert_tree_to_react_native(e, native_idents, builder));
                if element.children.is_empty() {
                    element.closing_element = None;
                }
                true
            }
            _ => false,
        },
        JSXChild::Fragment(fragment) => {
            fragment
                .children
                .retain_mut(|e| convert_tree_to_react_native(e, native_idents, builder));
            true
        }
        _ => true,
    }
}
