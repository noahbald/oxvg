use std::{ops::DerefMut, sync::LazyLock};

use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    visitor::Context,
};

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

    fn round_values<A: Attr>(
        &self,
        attr: &mut impl DerefMut<Target = A>,
    ) -> anyhow::Result<A::Atom> {
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
                self.get_mode()
                    .handle_non_roundable(value, &mut rounded_list);
                continue;
            };

            let mut number: f64 = captures.get(1).unwrap().as_str().parse()?;
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
                if float_precision > 0 {
                    number = number
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string();
                }
                if number.starts_with("0.") {
                    number.remove(0);
                } else if number.starts_with("-0.") {
                    number.remove(1);
                }
            }

            if default_px && matches!(unit, Some("px")) {
                unit = None;
            }

            rounded_list.push(number + unit.unwrap_or(""));
        }
        Ok(rounded_list.join(" ").into())
    }

    fn element<'arena, E: Element<'arena>>(
        &self,
        element: &E,
        _context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), String> {
        for mut attr in element.attributes().into_iter_mut() {
            if !self.get_mode().allowed_attribute(&attr) {
                continue;
            }

            log::debug!(
                "CleanupValues::run: rounding value: {:?}={}",
                attr.name(),
                attr.value()
            );
            match self.round_values(&mut attr) {
                Ok(new_value) => {
                    log::debug!(
                        "CleanupValues::run: rounded {:?} to {new_value}",
                        attr.name(),
                    );
                    attr.set_value(new_value);
                }
                Err(error) => {
                    log::debug!("CleanupValues::run: failed to round values: {error}");
                }
            };
        }
        Ok(())
    }
}

impl Mode {
    pub fn allowed_attribute(&self, attr: &impl DerefMut<Target = impl Attr>) -> bool {
        let name = attr.local_name();
        let name = name.as_ref();
        match self {
            Self::List => matches!(
                name,
                "points"
                    | "enable-background"
                    | "viewBox"
                    | "stroke-dasharray"
                    | "dx"
                    | "dy"
                    | "x"
                    | "y"
            ),
            // TODO: visitor for styles
            Self::SingleValue => !matches!(name, "version" | "style"),
        }
    }

    pub fn separate_value<'a>(
        &self,
        attr: &'a impl DerefMut<Target = impl Attr + 'a>,
    ) -> impl Iterator<Item = &'a str> {
        let value = attr.value().as_ref();
        if (matches!(self, Self::List) || attr.local_name().as_ref() == "viewBox") {
            Box::new(SEPARATOR.split(value)) as Box<dyn Iterator<Item = &str>>
        } else {
            Box::new(std::iter::once(value)) as Box<dyn Iterator<Item = &str>>
        }
    }

    fn handle_non_roundable(&self, value: &str, rounded_list: &mut Vec<String>) {
        match self {
            Self::SingleValue => rounded_list.push(value.to_string()),
            Self::List => {
                if value.contains("new") {
                    rounded_list.push("new".to_string());
                } else {
                    rounded_list.push(value.to_string());
                }
            }
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

static SEPARATOR: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\s+,?\s*|,\s*").unwrap());
static NUMERIC_VALUES: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^([-+]?\d*\.?\d+([eE][-+]?\d+)?)(px|pt|pc|mm|cm|m|in|ft|em|ex|%)?$")
        .unwrap()
});
