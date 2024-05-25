mod add_attributes_to_svg_element;
mod add_classes_to_svg;
mod cleanup_attributes;
mod cleanup_enable_background;

use std::rc::Rc;

use crate::configuration::Configuration;

pub use self::add_attributes_to_svg_element::AddAttributesToSVGElement;
pub use self::add_classes_to_svg::AddClassesToSVG;
pub use self::cleanup_attributes::CleanupAttributes;
pub use self::cleanup_enable_background::CleanupEnableBackground;

pub trait Job: Sized + Default {
    fn from_configuration(value: serde_json::Value) -> Self;

    fn prepare(&mut self, _document: &rcdom::RcDom) -> Option<()> {
        None
    }

    fn run(&self, _node: &Rc<rcdom::Node>) {}
}

pub enum Jobs {
    AddAttributesToSVGElement(AddAttributesToSVGElement),
}

impl Jobs {
    pub fn run(&self, node: &Rc<rcdom::Node>) {
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
