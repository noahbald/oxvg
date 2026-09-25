//! Implementation of SVGR using `swc_core`.
// Some of the implementation here is based off https://github.com/parcel-bundler/parcel/blob/v2/crates/html/src/jsx.rs
mod get_variables;
mod preset;

use crate::{
    Config,
    config::State,
    error::BuildError,
    utils::{attr_filter_map, attr_to_jsx_str},
};

use convert_case::{Case, Casing as _};
use oxvg_ast::{
    element::Element,
    is_element,
    node::{self, NodeData, Ref},
    remove_attribute,
};
use oxvg_collections::{attribute::Attr, content_type::ContentType};

use lightningcss::{
    printer::PrinterOptions,
    properties::{PropertyId, custom::CustomPropertyName},
    traits::ToCss as _,
    values::{length::LengthValue, percentage::DimensionPercentage},
};
use oxvg_serialize::ToValue as _;
use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{
        Expr, Ident, IdentName, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue,
        JSXClosingElement, JSXClosingFragment, JSXElement, JSXElementChild, JSXElementName,
        JSXEmptyExpr, JSXExpr, JSXExprContainer, JSXFragment, JSXOpeningElement,
        JSXOpeningFragment, JSXText, KeyValueProp, Lit, ObjectLit, Prop, PropName, PropOrSpread,
    },
};

pub use get_variables::{Variables, default_template};
pub use preset::preset;

/// Converts a node reference to a SWC representation of [`JSXElementChild`].
///
/// # Errors
///
/// If any of the [`BuildError`] variants occur.
pub fn to_jsx<'input>(
    node: Ref<'input, '_>,
    config: &Config,
    state: Option<&State>,
) -> Result<JSXElementChild, BuildError<'input>> {
    match &node.node_data {
        NodeData::Document | NodeData::Root => {
            let mut children = node
                .child_nodes_iter()
                .filter(|node| matches!(node.node_type(), node::Type::Element | node::Type::Text))
                .map(|node| to_jsx(node, config, state))
                .collect::<Result<Vec<_>, BuildError>>()?;

            if children.len() == 1 {
                Ok(children.remove(0))
            } else {
                Ok(JSXElementChild::JSXFragment(JSXFragment {
                    children,
                    span: DUMMY_SP,
                    opening: JSXOpeningFragment { span: DUMMY_SP },
                    closing: JSXClosingFragment { span: DUMMY_SP },
                }))
            }
        }
        NodeData::Element { .. } => match node
            .element()
            .as_ref()
            .ok_or(BuildError::Unreachable)
            .and_then(|element| element_to_jsx(element, config, state))
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
                Ok(JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::JSXEmptyExpr(JSXEmptyExpr { span: DUMMY_SP }),
                    span: DUMMY_SP,
                }))
            }
            Err(err) => Err(err),
        },
        NodeData::Style(style) => style
            .borrow()
            .0
            .to_css_string(PrinterOptions::default())
            .map_err(|_| BuildError::PrinterError)
            .map(|value| {
                let value: swc_core::atoms::Atom = value.into();
                JSXElementChild::JSXText(JSXText {
                    value: value.clone(),
                    raw: value,
                    span: DUMMY_SP,
                })
            }),
        NodeData::PI { value, .. } | NodeData::Text(value) => {
            if let Some(value) = value.borrow().as_ref() {
                let value: swc_core::atoms::Atom = value.trim().into();
                Ok(JSXElementChild::JSXText(JSXText {
                    value: value.clone(),
                    raw: value,
                    span: DUMMY_SP,
                }))
            } else {
                Ok(JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::JSXEmptyExpr(JSXEmptyExpr { span: DUMMY_SP }),
                    span: DUMMY_SP,
                }))
            }
        }
        NodeData::Comment(_) => Ok(JSXElementChild::JSXExprContainer(JSXExprContainer {
            expr: JSXExpr::JSXEmptyExpr(JSXEmptyExpr { span: DUMMY_SP }),
            span: DUMMY_SP,
        })),
    }
}

/// Converts an element reference to a SWC representation of [`JSXElementChild`].
///
/// # Errors
///
/// If any of the [`BuildError`] variants occur.
pub fn element_to_jsx<'input>(
    element: &Element<'input, '_>,
    config: &Config,
    state: Option<&State>,
) -> Result<JSXElementChild, BuildError<'input>> {
    let name = element.qual_name();
    if !name.prefix().is_empty() {
        return Err(BuildError::UnknownXMLPrefixElement(name.clone()));
    }
    if is_element!(element, Svg) {
        remove_attribute!(element, XMLNS);
    }

    let jsx_name = JSXElementName::Ident(Ident::new_no_ctxt(
        name.local_name().as_str().into(),
        DUMMY_SP,
    ));
    let opening = JSXOpeningElement {
        name: jsx_name.clone(),
        attrs: element
            .attributes()
            .into_iter()
            .filter_map(|a| attr_filter_map(&a, name, config, state, attr_to_jsx))
            .collect::<Result<Vec<_>, BuildError>>()?,
        self_closing: !element.has_child_nodes(),
        span: DUMMY_SP,
        type_args: None,
    };

    let children = element
        .child_nodes_iter()
        .map(|node| to_jsx(node, config, state))
        .collect::<Result<Vec<_>, BuildError>>()?;

    let closing = if element.has_child_nodes() {
        Some(JSXClosingElement {
            name: jsx_name,
            span: DUMMY_SP,
        })
    } else {
        None
    };

    Ok(JSXElementChild::JSXElement(Box::new(JSXElement {
        opening,
        children,
        closing,
        span: DUMMY_SP,
    })))
}

fn attr_to_jsx<'input>(attr: &Attr<'input>) -> Result<JSXAttrOrSpread, BuildError<'input>> {
    Ok(JSXAttrOrSpread::JSXAttr(JSXAttr {
        name: JSXAttrName::Ident(IdentName::new(
            attr_to_jsx_str(attr.name())?.as_str().into(),
            DUMMY_SP,
        )),
        span: DUMMY_SP,
        value: attr_value_to_svg(attr)?,
    }))
}

fn attr_value_to_svg<'input>(
    attr: &Attr<'input>,
) -> Result<Option<JSXAttrValue>, BuildError<'input>> {
    Ok(Some(match attr.value() {
        ContentType::TrueFalse(value) => {
            if value.0 {
                return Ok(None);
            }
            JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Lit(Lit::Bool(value.0.into())))),
            })
        }
        ContentType::TrueFalseUndefined(value) => {
            JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Lit(if let Some(bool) = &value.0 {
                    if bool.0 {
                        return Ok(None);
                    }
                    Lit::Bool(bool.0.into())
                } else {
                    Lit::Str("undefined".into())
                }))),
            })
        }
        ContentType::Number(value) => JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(Expr::Lit(Lit::Num((*value as f64).into())))),
        }),
        ContentType::Integer(value) => JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(Expr::Lit(Lit::Num((*value as f64).into())))),
        }),
        ContentType::LengthPercentage(ref value)
            if let DimensionPercentage::Dimension(LengthValue::Px(value)) = &value.0 =>
        {
            JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Lit(Lit::Num((*value as f64).into())))),
            })
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
                    Ok(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: if Ident::verify_symbol(&name).is_err() {
                            PropName::Str(name.into())
                        } else {
                            PropName::Ident(IdentName::new(name.into(), DUMMY_SP))
                        },
                        value: Box::new(
                            decl.value_to_css_string(PrinterOptions::default())
                                .map_err(|_| BuildError::PrinterError)?
                                .into(),
                        ),
                    }))))
                })
                .collect::<Result<Vec<_>, BuildError>>()?;
            JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Object(ObjectLit {
                    props,
                    span: DUMMY_SP,
                }))),
            })
        }
        value => JSXAttrValue::Str(
            value
                .to_value_string(PrinterOptions {
                    minify: true,
                    ..PrinterOptions::default()
                })
                .map_err(|_| BuildError::PrinterError)?
                .into(),
        ),
    }))
}
