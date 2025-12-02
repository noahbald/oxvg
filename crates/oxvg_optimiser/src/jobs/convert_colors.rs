#![allow(deprecated)]

use lightningcss::{values::color::CssColor, visit_types, visitor::Visit};
use oxvg_ast::{
    element::Element,
    is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    attribute::{core::Style, Attr},
    element::ElementId,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// How the type will be converted.
pub enum Method {
    #[default]
    /// Use lightningcss for conversion.
    ///
    /// Note, lightningcss is already used during parsing & serializing,
    /// so choosing this option effectively skips the job.
    Lightning,
    /// Convert all colors to `currentcolor`.
    CurrentColor,
    #[doc(hidden)]
    #[cfg(feature = "napi")]
    /// Compatibility option for NAPI
    // FIXME: force discriminated union to prevent NAPI from failing CI
    Napi(),
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(match self.method {
            Some(Method::CurrentColor) => PrepareOutcome::none,
            None | Some(Method::Lightning) => {
                // Colours in styles & attributes are already handle by lightnincss during parsing
                log::debug!("ConvertColors::prepare: skipping default behaviour");
                PrepareOutcome::skip
            }
            #[cfg(feature = "napi")]
            Some(Method::Napi()) => panic!("Napi variant is not allowed!"),
        })
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let is_masked = is_element!(element, Mask) || element.closest(&ElementId::Mask).is_some();

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
            Self::Lightning => {}
            #[cfg(feature = "napi")]
            Self::Napi() => panic!("Napi variant is not allowed!"),
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
