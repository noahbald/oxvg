mod add_attributes_to_svg_element;
mod add_classes_to_svg;

use crate::configuration::Configuration;

pub use self::add_attributes_to_svg_element::AddAttributesToSVGElement;
pub use self::add_classes_to_svg::AddClassesToSVG;

pub trait Job: Sized + Default {
    fn from_configuration(value: serde_json::Value) -> Self;

    fn run(&self, node: &rcdom::Node);
}

pub enum Jobs {
    AddAttributesToSVGElement(AddAttributesToSVGElement),
}

impl Jobs {
    pub fn run(&self, node: &rcdom::Node) {
        match self {
            Self::AddAttributesToSVGElement(job) => job,
        }
        .run(node);
    }
}

impl TryFrom<Configuration> for Jobs {
    type Error = ();

    fn try_from(value: Configuration) -> Result<Self, ()> {
        let (name, value) = match value {
            Configuration::Name(name) => (name, serde_json::Value::Null),
            Configuration::Configuration { name, value } => (name, value),
        };

        match name.as_str() {
            "AddAttributesToSVGElement" => Ok(Self::AddAttributesToSVGElement(
                AddAttributesToSVGElement::from_configuration(value),
            )),
            _ => Err(()),
        }
    }
}
