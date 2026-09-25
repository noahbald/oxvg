//! Implementation of SVGR using `oxc_ast`.
// Some of the implementation here is based off https://github.com/parcel-bundler/parcel/blob/v2/crates/html/src/jsx.rs
mod get_variables;
mod preset;

use std::ops::Range;

use crate::{
    Config,
    config::State,
    error::BuildError,
    utils::{attr_filter_map, attr_to_jsx_str},
};

use convert_case::{Case, Casing as _};
use oxc_allocator::{ArenaVec, Box, CloneIn as _, FromIn as _, GetAllocator};
use oxc_ast::{
    ast::{
        Expression, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild,
        JSXClosingElement, JSXClosingFragment, JSXElementName, JSXEmptyExpression, JSXExpression,
        JSXExpressionContainer, JSXFragment, JSXOpeningElement, JSXOpeningFragment, JSXText,
        NumberBase, ObjectPropertyKind, PropertyKey, PropertyKind, Span, Str,
    },
    builder::GetAstBuilder,
};
use oxc_syntax::identifier::is_identifier_name;
use oxvg_ast::{
    element::Element,
    is_element,
    node::{self, NodeData, Ref},
    remove_attribute,
};
use oxvg_collections::{atom::Atom, attribute::Attr, content_type::ContentType};

use lightningcss::{
    printer::PrinterOptions,
    properties::{PropertyId, custom::CustomPropertyName},
    traits::ToCss as _,
    values::{length::LengthValue, percentage::DimensionPercentage},
};
use oxvg_serialize::ToValue as _;

pub use get_variables::{Variables, default_template};
pub use preset::preset;

fn range_into(range: Range<usize>) -> Span {
    Span::new(range.start as u32, range.end as u32)
}

/// Converts a node reference to a SWC representation of [`JSXElementChild`].
///
/// # Errors
///
/// If any of the [`BuildError`] variants occur.
pub fn to_jsx<'alloc, 'input, B: GetAstBuilder<'alloc>>(
    node: Ref<'input, '_>,
    config: &Config,
    state: Option<&State>,
    builder: &B,
) -> Result<JSXChild<'alloc>, BuildError<'input>> {
    match &node.node_data {
        NodeData::Document | NodeData::Root => {
            let children_iter = node
                .child_nodes_iter()
                .filter(|node| matches!(node.node_type(), node::Type::Element | node::Type::Text))
                .map(|node| to_jsx(node, config, state, builder));
            let size_hint = children_iter.size_hint();
            let capacity = size_hint.1.unwrap_or(size_hint.0);
            let mut children = ArenaVec::with_capacity_in(capacity, builder.builder());
            for child in children_iter {
                children.push(child?);
            }

            if children.len() == 1 {
                Ok(children.remove(0))
            } else {
                Ok(JSXChild::Fragment(JSXFragment::boxed(
                    Span::default(),
                    JSXOpeningFragment::new(Span::default(), builder),
                    children,
                    JSXClosingFragment::new(Span::default(), builder),
                    builder,
                )))
            }
        }
        NodeData::Element { .. } => match node
            .element()
            .as_ref()
            .ok_or(BuildError::Unreachable)
            .and_then(|element| element_to_jsx(element, config, state, builder))
        {
            Ok(node) => Ok(node),
            // These errors are okay, just emit warning and drop the element.
            Err(BuildError::UnknownXMLPrefixElement(name)) => {
                if config.warn() {
                    eprintln!(
                        "Warning: dropped `{name}` attribute from {}",
                        if let Some(state) = state {
                            state.component_name.as_str()
                        } else {
                            "document"
                        }
                    );
                }
                Ok(JSXChild::ExpressionContainer(
                    JSXExpressionContainer::boxed(
                        Span::default(),
                        JSXExpression::EmptyExpression(JSXEmptyExpression::boxed(
                            Span::default(),
                            builder,
                        )),
                        builder,
                    ),
                ))
            }
            Err(err) => Err(err),
        },
        NodeData::Style(style) => style
            .borrow()
            .0
            .to_css_string(PrinterOptions::default())
            .map_err(|_| BuildError::PrinterError)
            .map(|value| {
                JSXChild::Text(JSXText::boxed(
                    node.range.clone().map(range_into).unwrap_or_default(),
                    Str::from_in(value, builder.builder().allocator()),
                    None,
                    builder,
                ))
            }),
        NodeData::PI { value, .. } | NodeData::Text(value) => {
            if let Some(value) = value.borrow().as_ref() {
                Ok(JSXChild::Text(JSXText::boxed(
                    node.range.clone().map(range_into).unwrap_or_default(),
                    Str::from_str_in(value.trim().into(), builder.builder()),
                    None,
                    builder,
                )))
            } else {
                Ok(JSXChild::ExpressionContainer(
                    JSXExpressionContainer::boxed(
                        Span::default(),
                        JSXExpression::EmptyExpression(JSXEmptyExpression::boxed(
                            Span::default(),
                            builder,
                        )),
                        builder,
                    ),
                ))
            }
        }
        NodeData::Comment(_) => Ok(JSXChild::ExpressionContainer(
            JSXExpressionContainer::boxed(
                Span::default(),
                JSXExpression::EmptyExpression(JSXEmptyExpression::boxed(Span::default(), builder)),
                builder,
            ),
        )),
    }
}

/// Converts an element reference to a SWC representation of [`JSXElementChild`].
///
/// # Errors
///
/// If any of the [`BuildError`] variants occur.
pub fn element_to_jsx<'alloc, 'input, B: GetAstBuilder<'alloc>>(
    element: &Element<'input, '_>,
    config: &Config,
    state: Option<&State>,
    builder: &B,
) -> Result<JSXChild<'alloc>, BuildError<'input>> {
    let builder = builder.builder();
    let name = element.qual_name();
    if !name.prefix().is_empty() {
        return Err(BuildError::UnknownXMLPrefixElement(name.clone()));
    }
    if is_element!(element, Svg) {
        remove_attribute!(element, XMLNS);
    }

    let jsx_name = JSXElementName::new_identifier(
        Span::default(),
        atom_to_str(element.local_name(), builder),
        builder,
    );

    let children = element
        .child_nodes_iter()
        .map(|node| to_jsx(node, config, state, builder))
        .collect::<Result<Vec<_>, BuildError>>()?;
    let children_is_empty = children.is_empty();

    Ok(JSXChild::new_element(
        Span::default(),
        JSXOpeningElement::boxed(
            Span::default(),
            jsx_name.clone_in(builder.builder().allocator()),
            Option::<Box<'alloc, _>>::None,
            ArenaVec::from_iter_in(
                element
                    .attributes()
                    .into_iter()
                    .filter_map(|a| {
                        attr_filter_map(&a, name, config, state, |a| attr_to_jsx(a, builder))
                    })
                    .collect::<Result<std::vec::Vec<_>, BuildError>>()?
                    .into_iter(),
                builder.builder(),
            ),
            builder,
        ),
        ArenaVec::from_iter_in(children.into_iter(), builder.builder()),
        if children_is_empty {
            None
        } else {
            Some(JSXClosingElement::new(Span::default(), jsx_name, builder))
        },
        builder,
    ))
}

fn attr_to_jsx<'input, 'alloc, B: GetAstBuilder<'alloc>>(
    attr: &Attr<'input>,
    builder: &B,
) -> Result<JSXAttributeItem<'alloc>, BuildError<'input>> {
    Ok(JSXAttributeItem::new_attribute(
        Span::default(),
        JSXAttributeName::new_identifier(
            Span::default(),
            atom_to_str(&attr_to_jsx_str(attr.name())?, builder.builder()),
            builder,
        ),
        attr_value_to_svg(attr, builder)?,
        builder,
    ))
}

fn attr_value_to_svg<'input, 'alloc, B: GetAstBuilder<'alloc>>(
    attr: &Attr<'input>,
    builder: &B,
) -> Result<Option<JSXAttributeValue<'alloc>>, BuildError<'input>> {
    Ok(Some(match attr.value() {
        ContentType::TrueFalse(value) => {
            if value.0 {
                return Ok(None);
            } else {
                JSXAttributeValue::new_expression_container(
                    Span::default(),
                    JSXExpression::new_boolean_literal(Span::default(), value.0, builder),
                    builder,
                )
            }
        }
        ContentType::TrueFalseUndefined(value) => {
            if let Some(bool) = &value.0 {
                if bool.0 {
                    return Ok(None);
                } else {
                    JSXAttributeValue::new_expression_container(
                        Span::default(),
                        JSXExpression::new_boolean_literal(Span::default(), bool.0, builder),
                        builder,
                    )
                }
            } else {
                JSXAttributeValue::new_string_literal(Span::default(), "undefined", None, builder)
            }
        }
        ContentType::Number(value) => JSXAttributeValue::new_expression_container(
            Span::default(),
            JSXExpression::new_numeric_literal(
                Span::default(),
                (*value).into(),
                None,
                NumberBase::Float,
                builder,
            ),
            builder,
        ),
        ContentType::Integer(value) => JSXAttributeValue::new_expression_container(
            Span::default(),
            JSXExpression::new_numeric_literal(
                Span::default(),
                (*value).into(),
                None,
                NumberBase::Decimal,
                builder,
            ),
            builder,
        ),
        ContentType::LengthPercentage(ref value)
            if let DimensionPercentage::Dimension(LengthValue::Px(value)) = &value.0 =>
        {
            JSXAttributeValue::new_expression_container(
                Span::default(),
                JSXExpression::new_numeric_literal(
                    Span::default(),
                    (*value).into(),
                    None,
                    NumberBase::Float,
                    builder,
                ),
                builder,
            )
        }
        ContentType::LengthPercentage(ref value)
            if let DimensionPercentage::Dimension(LengthValue::Px(value)) = &value.0 =>
        {
            JSXAttributeValue::new_expression_container(
                Span::default(),
                JSXExpression::new_numeric_literal(
                    Span::default(),
                    (*value).into(),
                    None,
                    NumberBase::Float,
                    builder,
                ),
                builder,
            )
        }
        ContentType::Style(value) => {
            let props = value
                .declarations
                .iter()
                .map(|decl| {
                    let name = match decl.property_id() {
                        PropertyId::Custom(CustomPropertyName::Custom(name)) => {
                            name.0.as_ref().to_string()
                        }
                        id => id.name().to_case(Case::Camel),
                    };
                    let name = Str::from_in(name, builder.builder().allocator());
                    Ok(ObjectPropertyKind::new_object_property(
                        Span::default(),
                        PropertyKind::Init,
                        if is_identifier_name(&name) {
                            PropertyKey::new_identifier(Span::default(), name, builder)
                        } else {
                            PropertyKey::new_string_literal(Span::default(), name, None, builder)
                        },
                        Expression::new_string_literal(
                            Span::default(),
                            Str::from_in(
                                decl.value_to_css_string(PrinterOptions::default())
                                    .map_err(|_| BuildError::PrinterError)?,
                                builder.builder().allocator(),
                            ),
                            None,
                            builder,
                        ),
                        false,
                        false,
                        false,
                        builder,
                    ))
                })
                .collect::<Result<std::vec::Vec<_>, BuildError>>()?;
            JSXAttributeValue::new_expression_container(
                Span::default(),
                JSXExpression::new_object_expression(
                    Span::default(),
                    ArenaVec::from_iter_in(props.into_iter(), builder.builder()),
                    builder,
                ),
                builder,
            )
        }
        value => JSXAttributeValue::new_string_literal(
            Span::default(),
            Str::from_in(
                value
                    .to_value_string(PrinterOptions {
                        minify: true,
                        ..PrinterOptions::default()
                    })
                    .map_err(|_| BuildError::PrinterError)?,
                builder.builder().allocator(),
            ),
            None,
            builder,
        ),
    }))
}

fn atom_to_str<'alloc, A: GetAllocator<'alloc>>(atom: &Atom<'_>, allocator: &A) -> Str<'alloc> {
    match atom {
        Atom::Static(atom) => Str::new_const(atom),
        Atom::Cow(cow) => Str::from_str_in(&*cow, allocator),
    }
}
