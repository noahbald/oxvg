#![allow(deprecated)]

use lightningcss::{values::color::CssColor, visit_types, visitor::Visit};
use oxvg_ast::{
    attribute::data::{core::Style, Attr},
    element::{data::ElementId, Element},
    visitor::{Context, ContextFlags, Info, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ConvertCase {
    Upper,
    Lower,
    #[doc(hidden)]
    #[cfg(feature = "napi")]
    /// Compatibility option for NAPI
    // FIXME: force discriminated union to prevent NAPI from failing CI
    Napi(),
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
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

#[cfg_attr(feature = "wasm", derive(Tsify))]
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
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub method: Option<Method>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for ConvertColors {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _info: &Info<'input, 'arena>,
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
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let is_masked =
            *element.qual_name() == ElementId::Mask || element.closest(&ElementId::Mask).is_some();

        let mut method = self.method.clone().unwrap_or_default();
        if is_masked && matches!(method, Method::CurrentColor) {
            method = Method::Lightning;
        }
        for mut attr in element.attributes().into_iter_mut() {
            if let Attr::Style(Style(style)) = &mut *attr {
                style.visit(&mut method).ok();
            } else {
                attr.value_mut().visit_color(|color| {
                    color.visit(&mut method).ok();
                });
            }
        }
        Ok(())
    }
}

impl<'input> lightningcss::visitor::Visitor<'input> for Method {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> lightningcss::visitor::VisitTypes {
        visit_types!(COLORS)
    }

    fn visit_color(&mut self, color: &mut CssColor) -> Result<(), Self::Error> {
        match self {
            Self::CurrentColor => *color = CssColor::CurrentColor,
            Self::Lightning | Self::Value { .. } => {}
        }
        Ok(())
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
