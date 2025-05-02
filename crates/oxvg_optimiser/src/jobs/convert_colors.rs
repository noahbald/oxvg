#![allow(deprecated)]

use std::mem;

use lightningcss::{
    error::PrinterError,
    printer::{Printer, PrinterOptions},
    properties::{
        custom::{TokenList, TokenOrValue},
        svg::SVGPaint,
        text::TextDecoration,
    },
    stylesheet::{ParserOptions, StyleAttribute},
    values::color::CssColor,
    visit_types,
};
use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    style::{PresentationAttr, UnparsedPresentationAttr},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ConvertCase {
    Upper,
    Lower,
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
/// How the type will be converted.
pub enum Method {
    /// Use lightningcss for conversion.
    #[default]
    Lightning,
    /// Convert all colors to `currentcolor`.
    CurrentColor,
    /// Options matching SVGO, for ease in migration.
    #[deprecated = "These options don't do anything and will likely be removed in the future."]
    Value {
        names_2_hex: bool,
        rgb_2_hex: bool,
        convert_case: Option<ConvertCase>,
        short_hex: bool,
        short_name: bool,
    },
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
/// Converts color references to their shortest equivalent.
///
/// Colors are minified using lightningcss' [minification](https://lightningcss.dev/minification.html#minify-colors).
///
/// # Differences to SVGO
///
/// There's fewer options for colour conversion in exchange for more effective conversions.
///
/// # Correctness
///
/// By default this job should never visually change the document.
///
/// If the [`Method::CurrentColor`] is used all colours will inherit their text colour, which
/// may be different to original.
///
/// # Errors
///
/// If lightningcss fails to parse or serialize CSS values.
pub struct ConvertColors {
    /// Specifies how colours should be converted.
    pub method: Option<Method>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for ConvertColors {
    type Error = String;

    fn prepare(
        &self,
        _document: &E,
        _info: &Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(match self.method {
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
        })
    }

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let mask_localname = &"mask".into();
        let is_masked = element.local_name() == mask_localname
            || element.closest_local(mask_localname).is_some();

        let mut method = self.method.clone().unwrap_or_default();
        if is_masked && matches!(method, Method::CurrentColor) {
            method = Method::Lightning;
        }
        for mut attr in element.attributes().into_iter_mut() {
            if attr.local_name().as_ref() == "style" {
                let style = attr.value().to_string();
                let style = StyleAttribute::parse(&style, ParserOptions::default());
                let mut style = match style {
                    Ok(result) => result,
                    Err(e) => {
                        log::debug!("failed to convert {}: {e}", attr.formatter());
                        continue;
                    }
                };

                method.convert_style(&mut style).ok();
                if let Ok(minified_style) = method.to_css(&style) {
                    attr.set_value(minified_style.into());
                }
            } else {
                let minified_value = if let Some(mut presentation) = attr.presentation() {
                    if method
                        .convert_presentation(&mut presentation)
                        .unwrap_or(false)
                    {
                        presentation
                            .value_to_css_string(PrinterOptions {
                                minify: true,
                                ..Default::default()
                            })
                            .ok()
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(minified_value) = minified_value {
                    attr.set_value(minified_value.into());
                }
            }
        }
        Ok(())
    }
}

impl Method {
    fn convert_style(&mut self, style: &mut StyleAttribute) -> Result<(), String> {
        use lightningcss::visitor::Visitor;

        log::debug!("Method::convert_style: doing a thing");
        // CurrentColor is the only case in which we need to mutate the source css
        if !matches!(self, Self::CurrentColor) {
            return Ok(());
        }

        self.visit_declaration_block(&mut style.declarations)
    }

    fn convert_presentation(&mut self, attr: &mut PresentationAttr) -> Result<bool, String> {
        use lightningcss::visitor::Visitor;

        match attr {
            PresentationAttr::Color(color)
            | PresentationAttr::Fill(SVGPaint::Color(color))
            | PresentationAttr::FloodColor(color)
            | PresentationAttr::LightingColor(color)
            | PresentationAttr::StopColor(color)
            | PresentationAttr::Stroke(SVGPaint::Color(color))
            | PresentationAttr::TextDecoration(TextDecoration { color, .. }) => {
                self.visit_color(color)?;
            }
            PresentationAttr::Unparsed(UnparsedPresentationAttr {
                value: TokenList(vec),
                ..
            }) => {
                vec.iter_mut()
                    .filter_map(|tl| match tl {
                        TokenOrValue::Color(color) => Some(color),
                        _ => None,
                    })
                    .try_for_each(|color| self.visit_color(color))?;
            }

            _ => return Ok(false),
        };
        Ok(true)
    }

    fn convert_color(&self, color: &mut CssColor) {
        match self {
            Self::CurrentColor => &mem::replace(color, CssColor::CurrentColor),
            Self::Lightning | Self::Value { .. } => color,
        };
    }

    fn to_css(&self, style: &StyleAttribute) -> Result<String, PrinterError> {
        let printer_options = PrinterOptions {
            minify: true,
            ..Default::default()
        };
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
        let mut dest = Printer::new(&mut s, printer_options);
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

impl lightningcss::visitor::Visitor<'_> for Method {
    type Error = String;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(COLORS)
    }

    fn visit_color(&mut self, color: &mut CssColor) -> Result<(), Self::Error> {
        self.convert_color(color);
        Ok(())
    }
}

// FIXME: use #[napi]
// https://github.com/napi-rs/napi-rs/issues/2585
#[cfg(feature = "napi")]
impl napi::bindgen_prelude::TypeName for ConvertCase {
    fn type_name() -> &'static str {
        "ConvertCase"
    }
    fn value_type() -> napi::ValueType {
        napi::ValueType::Object
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ToNapiValue for ConvertCase {
    unsafe fn to_napi_value(
        env: napi::bindgen_prelude::sys::napi_env,
        val: ConvertCase,
    ) -> napi::bindgen_prelude::Result<napi::bindgen_prelude::sys::napi_value> {
        let env_wrapper = napi::bindgen_prelude::Env::from(env);
        let mut obj = env_wrapper.create_object()?;
        match val {
            Self::Upper => obj.set("type", "Upper")?,
            Self::Lower {} => obj.set("type", "Lower")?,
        }
        napi::bindgen_prelude::Object::to_napi_value(env, obj)
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::FromNapiValue for ConvertCase {
    unsafe fn from_napi_value(
        env: napi::bindgen_prelude::sys::napi_env,
        napi_val: napi::bindgen_prelude::sys::napi_value,
    ) -> napi::bindgen_prelude::Result<Self> {
        let obj = napi::bindgen_prelude::Object::from_napi_value(env, napi_val)?;
        let r#type: String = obj
            .get("type")
            .map_err(|mut err| {
                err.reason = format!("{} on ConvertCase.type", err.reason,);
                err
            })?
            .ok_or_else(|| {
                napi::bindgen_prelude::Error::new(
                    napi::bindgen_prelude::Status::InvalidArg,
                    "Missing field `type`",
                )
            })?;
        let val = match r#type.as_str() {
            "Upper" => Self::Upper,
            "Lower" => Self::Lower,
            _ => {
                return Err(napi::bindgen_prelude::Error::new(
                    napi::bindgen_prelude::Status::InvalidArg,
                    format!("Unknown variant `{type}`",),
                ))
            }
        };
        Ok(val)
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ValidateNapiValue for ConvertCase {}

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
