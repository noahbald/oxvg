use oxvg_ast::{
    attribute::Attributes,
    element::Element,
    visitor::{Context, Visitor},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
/// The method for ordering xmlns attributes
pub enum XMLNSOrder {
    /// Sort xmlns attributes alphabetically
    Alphabetical,
    #[default]
    /// Keep xmlns attributes at the front of the list
    Front,
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
/// Sorts attributes into a predictable order.
///
/// This doesn't affect the size of a document but will likely improve readability
/// and compression of the document.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct SortAttrs {
    /// A list of attributes in a given order.
    pub order: Option<Vec<String>>,
    /// The method for ordering xmlns attributes
    pub xmlns_order: Option<XMLNSOrder>,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for SortAttrs {
    type Error = String;

    fn element(
        &self,
        element: &mut E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        let order = self.order.as_ref().unwrap_or_else(|| &DEFAULT_ORDER);
        let xmlns_order = self.xmlns_order.is_none() || self.xmlns_order == Some(XMLNSOrder::Front);
        element.attributes().sort(order, xmlns_order);

        Ok(())
    }
}

// FIXME: use #[napi]
// https://github.com/napi-rs/napi-rs/issues/2585
#[cfg(feature = "napi")]
impl napi::bindgen_prelude::TypeName for XMLNSOrder {
    fn type_name() -> &'static str {
        "XMLNSOrder"
    }
    fn value_type() -> napi::ValueType {
        napi::ValueType::Object
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ToNapiValue for XMLNSOrder {
    unsafe fn to_napi_value(
        env: napi::bindgen_prelude::sys::napi_env,
        val: XMLNSOrder,
    ) -> napi::bindgen_prelude::Result<napi::bindgen_prelude::sys::napi_value> {
        let env_wrapper = napi::bindgen_prelude::Env::from(env);
        let mut obj = env_wrapper.create_object()?;
        match val {
            Self::Alphabetical => obj.set("type", "Alphabetical")?,
            Self::Front {} => obj.set("type", "Front")?,
        }
        napi::bindgen_prelude::Object::to_napi_value(env, obj)
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::FromNapiValue for XMLNSOrder {
    unsafe fn from_napi_value(
        env: napi::bindgen_prelude::sys::napi_env,
        napi_val: napi::bindgen_prelude::sys::napi_value,
    ) -> napi::bindgen_prelude::Result<Self> {
        let obj = napi::bindgen_prelude::Object::from_napi_value(env, napi_val)?;
        let r#type: String = obj
            .get("type")
            .map_err(|mut err| {
                err.reason = format!("{} on XMLNSOrder.type", err.reason);
                err
            })?
            .ok_or_else(|| {
                napi::bindgen_prelude::Error::new(
                    napi::bindgen_prelude::Status::InvalidArg,
                    "Missing field `type`",
                )
            })?;
        let val = match r#type.as_str() {
            "Alphabetical" => Self::Alphabetical,
            "Front" => Self::Front,
            _ => {
                return Err(napi::bindgen_prelude::Error::new(
                    napi::bindgen_prelude::Status::InvalidArg,
                    format!("Unknown variant `{type}`"),
                ))
            }
        };
        Ok(val)
    }
}

#[cfg(feature = "napi")]
impl napi::bindgen_prelude::ValidateNapiValue for XMLNSOrder {}

lazy_static! {
    pub static ref DEFAULT_ORDER: Vec<String> = vec![
        String::from("id"),
        String::from("width"),
        String::from("height"),
        String::from("x"),
        String::from("x1"),
        String::from("x2"),
        String::from("y"),
        String::from("y1"),
        String::from("y2"),
        String::from("cx"),
        String::from("cy"),
        String::from("r"),
        String::from("fill"),
        String::from("stroke"),
        String::from("marker"),
        String::from("d"),
        String::from("points"),
    ];
}

#[test]
fn sort_attrs() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": {} }"#,
        Some(
            r#"<svg r="" b="" x2="" cx="" y1="" a="" y="" y2="" x1="" cy="" x="">
    <!-- sort according to default list alphabetically -->
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": {} }"#,
        Some(
            r#"<svg a="" fill-opacity="" stroke="" fill="" stroke-opacity="">
    <!-- sort derived attributes like fill and fill-opacity -->
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": {} }"#,
        Some(
            r#"<svg xmlns:editor2="link2" fill="" b="" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:editor1="link1" xmlns="" d="">
    <!-- put xmlns and namespace attributes before others by default -->
    <rect editor2:b="" editor1:b="" editor2:a="" editor1:a="" />
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "sortAttrs": { "xmlnsOrder": "alphabetical" } }"#,
        Some(
            r#"<svg foo="bar" xmlns="http://www.w3.org/2000/svg" height="10" baz="quux" width="10" hello="world">
    <!-- optionally sort xmlns attributes alphabetically -->
    <rect x="0" y="0" width="100" height="100" stroke-width="1" stroke-linejoin="round" fill="red" stroke="orange" xmlns="http://www.w3.org/2000/svg"/>
    test
</svg>"#
        ),
    )?);

    Ok(())
}
