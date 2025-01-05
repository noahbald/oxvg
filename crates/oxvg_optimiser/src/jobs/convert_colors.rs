use std::mem;

use lightningcss::{
    error::PrinterError,
    printer::{Printer, PrinterOptions},
    properties::{
        border::{BorderBlockColor, BorderInlineColor, GenericBorder},
        custom::{CustomProperty, TokenList, TokenOrValue},
        svg::SVGPaint,
        text::{TextDecoration, TextEmphasis},
        ui::{Caret, ColorOrAuto},
        Property,
    },
    stylesheet::{ParserOptions, StyleAttribute},
    values::color::CssColor,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use super::ContextFlags;

#[derive(Deserialize, Clone)]
pub enum ConvertCase {
    Upper,
    Lower,
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Method {
    #[default]
    Lightning,
    CurrentColor,
    /// WARN: These options don't do anything right now, but exist in SVGO and may reluctantly be
    /// implemented here too
    Value {
        names_2_hex: bool,
        rgb_2_hex: bool,
        convert_case: Option<ConvertCase>,
        short_hex: bool,
        short_name: bool,
    },
}

enum Color<'a> {
    Single(&'a mut CssColor),
    Many(Vec<&'a mut CssColor>),
    None,
}

#[derive(Deserialize, Default, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct ConvertColors {
    pub method: Option<Method>,
}

impl<E: Element> Visitor<E> for ConvertColors {
    type Error = String;

    fn prepare(&mut self, _document: &E,  _context_flags: &mut ContextFlags) -> PrepareOutcome {
        match self.method {
            Some(Method::Value {
                names_2_hex,
                rgb_2_hex,
                ref convert_case,
                short_hex,
                short_name,
            }) => {
                if names_2_hex || rgb_2_hex || convert_case.is_some() || short_hex || short_name {
                    PrepareOutcome::none
                } else {
                    log::debug!("ConvertColors::prepare: skipping useless config");
                    PrepareOutcome::skip
                }
            }
            _ => PrepareOutcome::none,
        }
    }

    fn element(&mut self, element: &mut E, _context: & mut Context<E>) -> Result<(), String> {
        let mask_localname = &"mask".into();
        let is_masked = &element.local_name() == mask_localname
            || element.closest_local(mask_localname).is_some();

        for mut attr in element.attributes().iter() {
            let is_style = attr.local_name() == "style".into();
            let style = if is_style {
                attr.value().to_string()
            } else {
                format!("{}:{}", attr.local_name(), attr.value())
            };
            let style = StyleAttribute::parse(&style, ParserOptions::default());
            let mut style = match style {
                Ok(result) => result,
                Err(e) => {
                    log::debug!("failed to convert {attr}: {e}");
                    continue;
                }
            };

            let mut method = self.method.clone().unwrap_or_default();
            if is_masked && matches!(method, Method::CurrentColor) {
                method = Method::Lightning;
            }
            method.convert_style(&mut style);
            let mut minified_style = method.to_css(&style).unwrap();
            if !is_style {
                if let Some((_, value)) = minified_style.split_once(':') {
                    minified_style = value.trim_start().to_string();
                }
            }
            attr.set_value(minified_style.into());
        }
        Ok(())
    }
}

impl Method {
    fn convert_style(&self, style: &mut StyleAttribute) {
        log::debug!("Method::convert_style: doing a thing");
        // CurrentColor is the only case in which we need to mutate the source css
        if !matches!(self, Self::CurrentColor) {
            return;
        }

        for declaration in style.declarations.iter_mut() {
            self.convert_property(declaration);
        }
    }

    fn convert_property(&self, property: &mut Property) {
        let mut color = Color::get_colors(property);
        match color {
            Color::Single(ref mut color) => self.convert_color(color),
            Color::Many(mut colors) => colors.iter_mut().for_each(|c| self.convert_color(c)),
            Color::None => {}
        };
    }

    fn convert_color(&self, color: &mut CssColor) {
        match self {
            Self::CurrentColor => &mem::replace(color, CssColor::CurrentColor),
            Self::Lightning | Self::Value { .. } => color,
        };
    }

    fn to_css(&self, style: &StyleAttribute) -> Result<String, PrinterError> {
        let printer_options = PrinterOptions::default();
        // NOTE: Useless destructure, maybe we'll use this in the future?
        let (..) = match self {
            Self::Value {
                names_2_hex,
                rgb_2_hex,
                convert_case,
                short_hex,
                short_name,
            } if !names_2_hex
                || !rgb_2_hex
                || convert_case.is_some()
                || !short_hex
                || !short_name =>
            {
                (names_2_hex, rgb_2_hex, convert_case, short_hex, short_name)
            }
            _ => return Ok(style.to_css(printer_options)?.code),
        };

        let mut s = String::with_capacity(1);
        let mut dest = Printer::new(&mut s, PrinterOptions::default());
        let len =
            style.declarations.declarations.len() + style.declarations.important_declarations.len();
        let mut i = 0;

        macro_rules! write {
            ($decls: expr, $important: literal) => {
                for decl in &$decls {
                    decl.to_css(&mut dest, $important)?;
                    // TODO: Intercept and ensure restrictions are met
                    // Is this even possible?
                    if i != len - 1 {
                        dest.write_char(';')?;
                        dest.whitespace()?;
                    }
                    i += 1;
                }
            };
        }

        write!(style.declarations.declarations, false);
        write!(style.declarations.important_declarations, true);
        todo!("Restrictions on color conversions are not supported");
    }
}

impl<'a> Color<'a> {
    fn get_colors(property: &'a mut Property) -> Self {
        match property {
            Property::BackgroundColor(color)
            | Property::Color(color)
            | Property::BorderTopColor(color)
            | Property::BorderBottomColor(color)
            | Property::BorderLeftColor(color)
            | Property::BorderRightColor(color)
            | Property::BorderBlockStartColor(color)
            | Property::BorderBlockEndColor(color)
            | Property::BorderInlineStartColor(color)
            | Property::BorderInlineEndColor(color)
            | Property::Border(GenericBorder { color, .. })
            | Property::BorderTop(GenericBorder { color, .. })
            | Property::BorderBottom(GenericBorder { color, .. })
            | Property::BorderLeft(GenericBorder { color, .. })
            | Property::BorderRight(GenericBorder { color, .. })
            | Property::BorderBlock(GenericBorder { color, .. })
            | Property::BorderBlockStart(GenericBorder { color, .. })
            | Property::BorderBlockEnd(GenericBorder { color, .. })
            | Property::BorderInline(GenericBorder { color, .. })
            | Property::BorderInlineStart(GenericBorder { color, .. })
            | Property::BorderInlineEnd(GenericBorder { color, .. })
            | Property::Outline(GenericBorder { color, .. })
            | Property::OutlineColor(color)
            | Property::TextDecorationColor(color, _)
            | Property::TextDecoration(TextDecoration { color, .. }, _)
            | Property::TextEmphasisColor(color, _)
            | Property::TextEmphasis(TextEmphasis { color, .. }, _)
            | Property::CaretColor(ColorOrAuto::Color(color))
            | Property::Caret(Caret {
                color: ColorOrAuto::Color(color),
                ..
            })
            | Property::Fill(SVGPaint::Color(color))
            | Property::Stroke(SVGPaint::Color(color)) => Color::Single(color),
            Property::Background(vec) => {
                Color::Many(vec.into_iter().map(|bg| &mut bg.color).collect())
            }
            Property::BoxShadow(vec, _) => {
                Color::Many(vec.into_iter().map(|bs| &mut bs.color).collect())
            }
            Property::BorderColor(border) => Color::Many(vec![
                &mut border.top,
                &mut border.right,
                &mut border.bottom,
                &mut border.left,
            ]),
            Property::BorderBlockColor(BorderBlockColor { start, end })
            | Property::BorderInlineColor(BorderInlineColor { start, end }) => {
                Color::Many(vec![start, end])
            }
            Property::TextShadow(vec) => {
                Color::Many(vec.into_iter().map(|ts| &mut ts.color).collect())
            }
            Property::Custom(CustomProperty {
                value: TokenList(vec),
                ..
            }) => Color::Many(
                vec.iter_mut()
                    .filter_map(|tl| match tl {
                        TokenOrValue::Color(color) => Some(color),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => Color::None,
        }
    }
}

#[test]
fn convert_colors() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertColors": {  } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to hex -->
    <g color="black"/>
    <g color="BLACK"/>
    <path fill="rgb(64 64 64)"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertColors": {  } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to short hex -->
    <g color="#ff00aa"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertColors": { } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to named color -->
    <g color="#FF0000"/>
    <g color="#f00"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertColors": { "method": "currentColor" } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to currentColor -->
    <g color="black"/>
    <g color="BLACK"/>
    <g color="none"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
    <path fill="none"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertColors": { } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should preserve color-like substrings that aren't colors -->
    <linearGradient id="Aa">
        <stop stop-color="ReD" offset="5%"/>
    </linearGradient>
    <text x="0" y="32" fill="gold">uwu</text>
    <text x="0" y="64" fill="GOLD">owo</text>
    <text x="0" y="96" fill="url(#Aa)">eue</text>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertColors": { "method": "currentColor" } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should not apply `currentColor` to masks -->
    <path fill="currentcolor"/>
    <mask id="mask1" fill="#fff"/>
    <mask id="mask2">
        <path fill="rgba(255,255,255,0.75)"/>
    </mask>
    <mask id="mask3">
        <g>
            <path fill="#fff"/>
            <path stroke="#000"/>
        </g>
        <mask id="inner-mask" fill="rgba(0,0,0,.5)"/>
    </mask>
    <path fill="currentcolor"/>
</svg>"##
        )
    )?);

    Ok(())
}
