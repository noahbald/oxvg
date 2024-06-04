use std::rc::Rc;

use markup5ever::{local_name, tendril::StrTendril, Attribute};
use serde::Deserialize;

use crate::Job;

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanupListOfValues {
    float_precision: Option<usize>,
    leading_zero: Option<bool>,
    default_px: Option<bool>,
    convert_to_px: Option<bool>,
}

impl Job for CleanupListOfValues {
    fn run(&self, node: &Rc<rcdom::Node>) {
        use rcdom::NodeData::Element;

        let Element { attrs, .. } = &node.data else {
            return;
        };

        for attr in attrs.borrow_mut().iter_mut() {
            let name = &attr.name.local;
            if !(&local_name!("points") == name
                || &local_name!("enable-background") == name
                || &local_name!("viewBox") == name
                || &local_name!("stroke-dasharray") == name
                || &local_name!("dx") == name
                || &local_name!("dy") == name
                || &local_name!("x") == name
                || &local_name!("y") == name
                // NOTE: This differs from SVGO, which doesn't include `d`
                || &local_name!("d") == name)
            {
                continue;
            }

            match self.round_values(attr) {
                Ok(new_value) => {
                    dbg!(&new_value);
                    attr.value = new_value;
                }
                Err(error) => {
                    dbg!(error);
                }
            };
        }
    }
}

impl CleanupListOfValues {
    fn round_values(&self, attr: &mut Attribute) -> anyhow::Result<StrTendril> {
        let mut rounded_list: Vec<String> = Vec::new();
        let float_precision = self.float_precision.unwrap_or(DEFAULT_FLOAT_PRECISION);
        let do_convert_to_px = self.convert_to_px.unwrap_or(DEFAULT_CONVERT_TO_PX);
        let leading_zero = self.leading_zero.unwrap_or(DEFAULT_LEADING_ZERO);
        let default_px = self.default_px.unwrap_or(DEFAULT_DEFAULT_PX);

        for value in SEPARATOR.split(&attr.value) {
            let Some(captures) = NUMERIC_VALUES.captures(value) else {
                if value.contains("new") {
                    rounded_list.push("new".to_string());
                } else {
                    rounded_list.push(value.to_string());
                }
                continue;
            };

            let mut number: f64 = captures.get(1).unwrap().as_str().parse()?;
            let exponent = captures.get(2).map(|capture| capture.as_str());
            let mut unit = captures.get(3).map(|capture| capture.as_str());
            if do_convert_to_px {
                if let Some(unwrapped_unit) = unit {
                    let new_number = convert_to_px(number, unwrapped_unit);
                    if (new_number - number).abs() > f64::EPSILON {
                        unit = Some("px");
                        number = new_number;
                    }
                }
            }

            let mut number = format!("{number:.float_precision$}");
            if leading_zero {
                number = number
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string();
            }

            if default_px && matches!(unit, Some("px")) {
                unit = None;
            }

            rounded_list.push(number + exponent.unwrap_or("") + unit.unwrap_or(""));
        }
        Ok(rounded_list.join(" ").into())
    }
}

fn convert_to_px(number: f64, unit: &str) -> f64 {
    number
        * match unit {
            "cm" => 96.0 / 2.54,
            "mm" => 96.0 / 25.4,
            "in" => 96.0,
            "pt" => 4.0 / 3.0,
            "pc" => 16.0,
            _ => 1.0,
        }
}

static DEFAULT_FLOAT_PRECISION: usize = 3;
static DEFAULT_LEADING_ZERO: bool = true;
static DEFAULT_DEFAULT_PX: bool = true;
static DEFAULT_CONVERT_TO_PX: bool = true;

lazy_static! {
    static ref SEPARATOR: regex::Regex = regex::Regex::new(r"\s+,?\s*|,\s*").unwrap();
    static ref NUMERIC_VALUES: regex::Regex =
        regex::Regex::new(r"^([-+]?\d*\.?\d+([eE][-+]?\d+)?)(px|pt|pc|mm|cm|m|in|ft|em|ex|%)?$")
            .unwrap();
}

#[test]
fn cleanup_list_of_values() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        // Should round values, maintaining non-numerical values
        r#"{ "cleanupListOfValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500.2132 500.213823642" enable-background="new 0 0 500.224551535 500.213262">
    test
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        // Should cleanup polygon points
        r#"{ "cleanupListOfValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <polygon stroke-dasharray="200.22222 200.22522" points="413.74712,290.95212 290.75632  ,  343.89942 183.40744 ,263.8582523 199.05334,  130.871345 322.04442,77.92533 429.39122,157.96555 "/>
    test
    <g fill="none" stroke-dasharray="8, 8" stroke-width="3">
        <path d="M83 250c69-18 140-40 197-84 33-23 48-62 62-99 2-6 3-12 7-16"/>
        <path stroke-dasharray="none" stroke-linejoin="round" d="M83 250c29-34 57-72 97-94 33-13 69-10 104-11 22 1 45 2 65 13"/>
    </g>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        // Should cleanup x/y values
        r#"{ "cleanupListOfValues": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <polygon stroke-dasharray="200.22222 200.22522" points="413.74712,290.95212 290.75632  ,  343.89942 183.40744 ,263.8582523 199.05334,  130.871345 322.04442,77.92533 429.39122,157.96555 "/>
    test
    <g fill="none" stroke-dasharray="8, 8" stroke-width="3">
        <path d="M83 250c69-18 140-40 197-84 33-23 48-62 62-99 2-6 3-12 7-16"/>
        <path stroke-dasharray="none" stroke-linejoin="round" d="M83 250c29-34 57-72 97-94 33-13 69-10 104-11 22 1 45 2 65 13"/>
    </g>
</svg>"#
        )
    )?);

    Ok(())
}
