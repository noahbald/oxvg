use std::rc::Rc;

use markup5ever::{local_name, tendril::StrTendril, Attribute};

use crate::{Context, Job};

pub struct Options {
    pub float_precision: usize,
    pub leading_zero: bool,
    pub default_px: bool,
    pub do_convert_to_px: bool,
}

#[derive(Default, Clone)]
pub enum Mode {
    #[default]
    List,
    SingleValue,
}

pub trait CleanupValues {
    fn get_options(&self) -> Options;

    fn get_mode(&self) -> Mode;

    fn round_values(&self, attr: &mut Attribute) -> anyhow::Result<StrTendril> {
        let mut rounded_list: Vec<String> = Vec::new();
        let Options {
            float_precision,
            do_convert_to_px,
            leading_zero,
            default_px,
        } = self.get_options();

        for value in self.get_mode().separate_value(attr) {
            if value.is_empty() {
                continue;
            }

            let Some(captures) = NUMERIC_VALUES.captures(&value) else {
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
                if number.starts_with("0.") {
                    number.remove(0);
                } else if number.starts_with("-0.") {
                    number.remove(1);
                }
            }

            if default_px && matches!(unit, Some("px")) {
                unit = None;
            }

            rounded_list.push(number + exponent.unwrap_or("") + unit.unwrap_or(""));
        }
        Ok(rounded_list.join(" ").into())
    }
}

impl<T: CleanupValues> Job for T {
    fn run(&self, node: &Rc<rcdom::Node>, _context: &Context) {
        use rcdom::NodeData::Element;

        let Element { attrs, .. } = &node.data else {
            return;
        };

        for attr in attrs.borrow_mut().iter_mut() {
            if !self.get_mode().allowed_attribute(attr) {
                continue;
            }

            match self.round_values(attr) {
                Ok(new_value) => {
                    dbg!(
                        "CleanupValues::run: rounding value",
                        &attr.name.local,
                        &attr.value,
                        &new_value
                    );
                    attr.value = new_value;
                }
                Err(error) => {
                    dbg!(error);
                }
            };
        }
    }
}

impl Mode {
    pub fn allowed_attribute(&self, attr: &Attribute) -> bool {
        let name = &attr.name.local;
        match self {
            Self::List => {
                &local_name!("points") == name
                || &local_name!("enable-background") == name
                || &local_name!("viewBox") == name
                || &local_name!("stroke-dasharray") == name
                || &local_name!("dx") == name
                || &local_name!("dy") == name
                || &local_name!("x") == name
                || &local_name!("y") == name
                // WARN: This differs from SVGO, which doesn't include `d`
                || &local_name!("d") == name
            }
            Self::SingleValue => &local_name!("version") != name,
        }
    }

    pub fn separate_value<'a>(&'a self, attr: &'a Attribute) -> impl Iterator<Item = String> + 'a {
        if (matches!(self, Self::List) || attr.name.local == local_name!("viewBox")) {
            Box::new(
                SEPARATOR
                    .split(&attr.value)
                    .map(std::string::ToString::to_string),
            ) as Box<dyn Iterator<Item = String>>
        } else {
            Box::new(std::iter::once(attr.value.to_string())) as Box<dyn Iterator<Item = String>>
        }
    }
}

pub fn convert_to_px(number: f64, unit: &str) -> f64 {
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

lazy_static! {
    static ref SEPARATOR: regex::Regex = regex::Regex::new(r"\s+,?\s*|,\s*").unwrap();
    static ref NUMERIC_VALUES: regex::Regex =
        regex::Regex::new(r"^([-+]?\d*\.?\d+([eE][-+]?\d+)?)(px|pt|pc|mm|cm|m|in|ft|em|ex|%)?$")
            .unwrap();
}
