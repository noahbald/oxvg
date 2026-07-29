//! A reimplementation of `@svgr/babel-preset` and it's `@svgr` dependencies in Rust.
use std::collections::HashSet;

use swc_core::{
    common::{BytePos, DUMMY_SP},
    ecma::ast::{
        BinExpr, BinaryOp, CondExpr, EsVersion, Expr, IdentName, JSXAttr, JSXAttrName,
        JSXAttrOrSpread, JSXAttrValue, JSXClosingElement, JSXElement, JSXElementChild,
        JSXElementName, JSXEmptyExpr, JSXExpr, JSXExprContainer, JSXOpeningElement, Lit, Null,
        Number, SpreadElement,
    },
};
use swc_ecma_lexer::{common::parser::Parser as _, Lexer, Parser, StringInput, Syntax};

use crate::{
    config::{ExpandProps, Icon},
    error::{ConfigError, Error},
    BuildError, Config,
};

/// Mutates the given root JSX element based on the given config.
///
/// # Errors
///
/// If there is an error building the document or parsing the config.
#[allow(clippy::too_many_lines)]
pub fn preset<'a, S: std::hash::BuildHasher>(
    jsx: &mut JSXElementChild,
    config: &Config,
    native_idents: &mut HashSet<&'static str, S>,
) -> Result<(), Error<'a>> {
    if config.native() {
        convert_tree_to_react_native(jsx, native_idents);
    }
    let ident_svg = if config.native() { "Svg" } else { "svg" };
    let svg_element = get_first_named_jsx_element(jsx, ident_svg)
        .ok_or(BuildError::MissingSVGElement)
        .map_err(Error::BuildError)?;

    if let Some(svg_props) = &config.svg_props {
        for (key, value) in svg_props {
            let attr = parse_templatable_attr(value).map_err(Error::ConfigError)?;
            set_attribute(svg_element, key, attr, true);
        }
    }

    if config.r#ref() {
        set_attribute(
            svg_element,
            "ref",
            JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Ident("ref".into()))),
            }),
            true,
        );
    }

    if config.title_prop() {
        config.title_prop();
        set_attribute(
            svg_element,
            "aria-labelledby",
            JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Ident("titleId".into()))),
            }),
            true,
        );
    }

    if config.desc_prop() {
        set_attribute(
            svg_element,
            "aria-describedby",
            JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Ident("descId".into()))),
            }),
            true,
        );
    }

    if !config.dimensions() {
        svg_element.opening.attrs.retain(|attr| {
            if let JSXAttrOrSpread::JSXAttr(JSXAttr {
                name: JSXAttrName::Ident(attr),
                ..
            }) = attr
            {
                !matches!(attr.sym.as_str(), "width" | "height")
            } else {
                true
            }
        });
    }

    match &config.icon {
        Some(Icon::Bool(true)) => {
            let attr = if config.native() {
                JSXAttrValue::JSXExprContainer(JSXExprContainer {
                    span: DUMMY_SP,
                    expr: JSXExpr::Expr(Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 24.0,
                        raw: None,
                    })))),
                })
            } else {
                JSXAttrValue::Str("1em".into())
            };
            set_attribute(svg_element, "width", attr.clone(), true);
            set_attribute(svg_element, "height", attr, true);
        }
        Some(Icon::String(str)) => {
            let str = str.as_str();
            set_attribute(svg_element, "width", JSXAttrValue::Str(str.into()), true);
            set_attribute(svg_element, "height", JSXAttrValue::Str(str.into()), true);
        }
        Some(Icon::Number(value)) => {
            let attr = JSXAttrValue::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: *value,
                    raw: None,
                })))),
            });
            set_attribute(svg_element, "width", attr.clone(), true);
            set_attribute(svg_element, "height", attr, true);
        }
        Some(Icon::Bool(false)) | None => {}
    }

    // WARN: No pushing props after this!
    //       e.g. set_attribute_recursive(..., true)
    let element = JSXAttrOrSpread::SpreadElement(SpreadElement {
        dot3_token: DUMMY_SP,
        expr: Box::new(Expr::Ident("props".into())),
    });
    match config.expand_props() {
        ExpandProps::Start => svg_element.opening.attrs.insert(0, element),
        ExpandProps::End => svg_element.opening.attrs.push(element),
        ExpandProps::None => {}
    }

    if let Some(replace_attr_values) = &config.replace_attr_values {
        for (key, value) in replace_attr_values {
            let attr = parse_templatable_attr(value).map_err(Error::ConfigError)?;
            set_value_recursive(jsx, key, &attr);
        }
    }

    if config.desc_prop() {
        let ident_desc = if config.native() { "Desc" } else { "desc" };
        replace_element_conditionally(jsx, ident_desc, "descId", "desc", ident_svg)
            .map_err(Error::BuildError)?;
    }

    if config.title_prop() {
        let ident_title = if config.native() { "Title" } else { "title" };
        replace_element_conditionally(jsx, ident_title, "titleId", "title", ident_svg)
            .map_err(Error::BuildError)?;
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn replace_element_conditionally(
    jsx: &mut JSXElementChild,
    tag: &str,
    id: &str,
    ident_child: &str,
    ident_root: &str,
) -> Result<(), BuildError<'static>> {
    let existing = get_first_named_jsx_element_child(jsx, tag);
    let mut existing_id = None;
    let existing = if let Some(existing) = existing {
        let mut original_title = std::mem::replace(
            existing,
            JSXElementChild::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::JSXEmptyExpr(JSXEmptyExpr { span: DUMMY_SP }),
            }),
        );
        if let JSXElementChild::JSXElement(element) = &mut original_title {
            if let Some(JSXAttr { value, .. }) = element
                .opening
                .attrs
                .iter_mut()
                .filter_map(|a| {
                    if let JSXAttrOrSpread::JSXAttr(a) = a {
                        Some(a)
                    } else {
                        None
                    }
                })
                .find(|a| {
                    if let JSXAttrName::Ident(name) = &a.name {
                        name.sym == "id"
                    } else {
                        false
                    }
                })
            {
                match value {
                    Some(JSXAttrValue::Str(s)) => {
                        *value = Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                            span: DUMMY_SP,
                            expr: JSXExpr::Expr(Box::new(Expr::Bin(BinExpr {
                                span: DUMMY_SP,
                                op: BinaryOp::LogicalOr,
                                left: Box::new(Expr::Ident(id.into())),
                                right: Box::new(Expr::Lit(Lit::Str(s.clone()))),
                            }))),
                        }));
                        existing_id.clone_from(value);
                    }
                    Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                        expr: JSXExpr::Expr(e),
                        ..
                    })) => {
                        *value = Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                            span: DUMMY_SP,
                            expr: JSXExpr::Expr(Box::new(Expr::Bin(BinExpr {
                                span: DUMMY_SP,
                                op: BinaryOp::LogicalOr,
                                left: Box::new(Expr::Ident(id.into())),
                                right: e.clone(),
                            }))),
                        }));
                        existing_id.clone_from(value);
                    }
                    _ => *value = Some(JSXAttrValue::Str(id.into())),
                }
            }
        }
        Some((existing, original_title))
    } else {
        None
    };
    let inserting_element = Box::new(Expr::JSXElement(Box::new(JSXElement {
        span: DUMMY_SP,
        opening: JSXOpeningElement {
            span: DUMMY_SP,
            name: JSXElementName::Ident(tag.into()),
            attrs: vec![JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "id".into(),
                }),
                value: Some(existing_id.unwrap_or_else(|| {
                    JSXAttrValue::JSXExprContainer(JSXExprContainer {
                        span: DUMMY_SP,
                        expr: JSXExpr::Expr(Box::new(Expr::Ident(id.into()))),
                    })
                })),
            })],
            self_closing: false,
            type_args: None,
        },
        closing: Some(JSXClosingElement {
            span: DUMMY_SP,
            name: JSXElementName::Ident(tag.into()),
        }),
        children: vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(Expr::Ident(ident_child.into()))),
        })],
    })));
    if let Some((existing, original_title)) = existing {
        if let JSXElementChild::JSXElement(inner) = original_title {
            *existing = JSXElementChild::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Cond(CondExpr {
                    span: DUMMY_SP,
                    test: Box::new(Expr::Bin(BinExpr {
                        span: DUMMY_SP,
                        op: BinaryOp::EqEqEq,
                        left: Box::new(Expr::Ident(ident_child.into())),
                        right: Box::new(Expr::Ident("undefined".into())),
                    })),
                    cons: Box::new(Expr::JSXElement(inner)),
                    alt: inserting_element,
                }))),
            });
        } else {
            *existing = original_title;
        }
    } else {
        drop(existing);
        let svg_element =
            get_first_named_jsx_element(jsx, ident_root).ok_or(BuildError::MissingSVGElement)?;
        svg_element.children.insert(
            0,
            JSXElementChild::JSXExprContainer(JSXExprContainer {
                span: DUMMY_SP,
                expr: JSXExpr::Expr(Box::new(Expr::Cond(CondExpr {
                    span: DUMMY_SP,
                    test: Box::new(Expr::Ident(ident_child.into())),
                    cons: inserting_element,
                    alt: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                }))),
            }),
        );
        svg_element.opening.self_closing = false;
        svg_element.closing = Some(JSXClosingElement {
            span: DUMMY_SP,
            name: svg_element.opening.name.clone(),
        });
    }
    Ok(())
}

fn get_first_named_jsx_element_child<'a>(
    jsx: &'a mut JSXElementChild,
    name: &str,
) -> Option<&'a mut JSXElementChild> {
    let matched = match &*jsx {
        JSXElementChild::JSXElement(element) => matches!(
            &element.opening.name,
            JSXElementName::Ident(ident) if ident.sym == name,
        ),
        _ => false,
    };
    if matched {
        return Some(jsx);
    }
    match jsx {
        JSXElementChild::JSXElement(element) => {
            for child in &mut element.children {
                if let Some(child) = get_first_named_jsx_element_child(child, name) {
                    return Some(child);
                }
            }
            None
        }
        JSXElementChild::JSXFragment(element) => {
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

fn get_first_named_jsx_element<'a>(
    jsx: &'a mut JSXElementChild,
    name: &str,
) -> Option<&'a mut Box<JSXElement>> {
    match jsx {
        JSXElementChild::JSXElement(element) => match &element.opening.name {
            JSXElementName::Ident(ident) if ident.sym == name => Some(element),
            _ => {
                for child in &mut element.children {
                    if let Some(child) = get_first_named_jsx_element(child, name) {
                        return Some(child);
                    }
                }
                None
            }
        },
        JSXElementChild::JSXFragment(element) => {
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

fn parse_templatable_attr(template: &str) -> Result<JSXAttrValue, ConfigError> {
    if template.starts_with('{') && template.ends_with('}') {
        let input = &template[1..template.len() - 1];
        let lexer = Lexer::new(
            Syntax::default(),
            EsVersion::default(),
            StringInput::new(input, BytePos(0), BytePos(input.len().max(1) as u32)),
            None,
        );
        let mut parser = Parser::new_from(lexer);
        let expr = parser
            .parse_expr()
            .map_err(|err| err.into_kind().msg().into())
            .map_err(ConfigError::InvalidExpr)?;
        Ok(JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(expr),
        }))
    } else {
        Ok(JSXAttrValue::Str(template.into()))
    }
}

fn set_value_recursive(root: &mut JSXElementChild, old_value: &str, new_value: &JSXAttrValue) {
    match root {
        JSXElementChild::JSXElement(element) => {
            for attr in &mut element.opening.attrs {
                if let JSXAttrOrSpread::JSXAttr(JSXAttr {
                    value: Some(value), ..
                }) = attr
                {
                    if let JSXAttrValue::Str(str) = &value {
                        if str.value.as_bytes() == old_value.as_bytes() {
                            *value = new_value.clone();
                        }
                    }
                }
            }
            for child in &mut element.children {
                set_value_recursive(child, old_value, new_value);
            }
        }
        JSXElementChild::JSXFragment(fragment) => {
            for child in &mut fragment.children {
                set_value_recursive(child, old_value, new_value);
            }
        }
        _ => {}
    }
}

fn set_attribute(
    element: &mut Box<JSXElement>,
    name: &str,
    new_value: JSXAttrValue,
    allow_push: bool,
) {
    for attr in &mut element.opening.attrs {
        if let JSXAttrOrSpread::JSXAttr(JSXAttr {
            name: JSXAttrName::Ident(ident),
            value,
            ..
        }) = attr
        {
            if ident.sym == name {
                *value = Some(new_value);
                return;
            }
        }
    }
    if allow_push {
        element
            .opening
            .attrs
            .push(JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(name.into()),
                value: Some(new_value),
            }));
    }
}

fn convert_tree_to_react_native<S: std::hash::BuildHasher>(
    jsx: &mut JSXElementChild,
    native_idents: &mut HashSet<&'static str, S>,
) -> bool {
    match jsx {
        JSXElementChild::JSXElement(element) => match &mut element.opening.name {
            JSXElementName::Ident(name) => {
                let new_name = match name.sym.as_str() {
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
                *name = new_name.into();
                if let Some(JSXClosingElement { name, .. }) = &mut element.closing {
                    *name = JSXElementName::Ident(new_name.into());
                }
                element
                    .children
                    .retain_mut(|e| convert_tree_to_react_native(e, native_idents));
                if element.children.is_empty() {
                    element.closing = None;
                }
                true
            }
            _ => false,
        },
        JSXElementChild::JSXFragment(fragment) => {
            fragment
                .children
                .retain_mut(|e| convert_tree_to_react_native(e, native_idents));
            true
        }
        _ => true,
    }
}
