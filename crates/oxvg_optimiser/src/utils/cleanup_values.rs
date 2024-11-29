use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
};

use crate::{Context, Job, JobDefault};

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

pub trait CleanupValues: JobDefault {
    fn get_options(&self) -> Options;

    fn get_mode(&self) -> Mode;

    fn round_values<A: Attr>(&self, attr: &mut A) -> anyhow::Result<A::Atom> {
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
    fn run<E: Element>(&self, element: &E, _context: &Context) {
        for mut attr in element.attributes().iter() {
            if !self.get_mode().allowed_attribute(&attr) {
                continue;
            }

            let name = attr.name();
            let value = attr.value();
            match self.round_values(&mut attr) {
                Ok(new_value) => {
                    log::debug!(
                        "CleanupValues::run: rounding value: {name:?}={value} -> {new_value}"
                    );
                    attr.set_value(new_value);
                }
                Err(error) => {
                    log::debug!("CleanupValues::run: failed to round values: {error}");
                }
            };
        }
    }
}

impl Mode {
    pub fn allowed_attribute(&self, attr: &impl Attr) -> bool {
        let name = attr.local_name();
        let name = name.as_ref();
        match self {
            Self::List => {
                "points" == name
                || "enable-background" == name
                || "viewBox" == name
                || "stroke-dasharray" == name
                || "dx" == name
                || "dy" == name
                || "x" == name
                || "y" == name
                // WARN: This differs from SVGO, which doesn't include `d`
                || "d" == name
            }
            Self::SingleValue => "version" != name,
        }
    }

    pub fn separate_value<'a>(&self, attr: &'a impl Attr) -> impl Iterator<Item = &'a str> {
        let value = attr.value_ref();
        if (matches!(self, Self::List) || attr.local_name() == "viewBox".into()) {
            Box::new(SEPARATOR.split(value)) as Box<dyn Iterator<Item = &str>>
        } else {
            Box::new(std::iter::once(value)) as Box<dyn Iterator<Item = &str>>
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
