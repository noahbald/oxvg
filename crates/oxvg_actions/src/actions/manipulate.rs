mod path;

use lightningcss::{declaration::DeclarationBlock, traits::Parse};
use oxvg_ast::{get_attribute_mut, set_attribute};
use oxvg_collections::{
    atom::Atom,
    attribute::{Attr, AttrId, core_attrs::Style},
};

use crate::{Action, Actor, Error};

impl<'input> Actor<'input, '_> {
    /// Sets the attribute to selected elements.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/attr.md")]
    pub fn attr(&mut self, name: &str, value: &str) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Attr {
            name: name.to_string().into(),
            value: value.to_string().into(),
        });

        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        for selection in selections {
            #[allow(clippy::cast_sign_loss)]
            let Some(node) = self.allocator.get(selection as usize) else {
                continue;
            };
            let Some(element) = node.element() else {
                continue;
            };
            let attr = element.parse_attr_id(name);
            if matches!(attr, AttrId::Unknown(_)) {
                continue;
            }

            let value = self.allocator.alloc_str(value);
            let attr = Attr::new(attr, value);
            element.set_attribute(attr);
        }

        self.effect_document()
    }

    /// Toggles the class-name on selected elements.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/class.md")]
    pub fn class(&mut self, name: &str) -> Result<(), Error<'input>> {
        let name: Atom<'static> = name.to_string().into();
        self.effect_history(&Action::Class(name.clone()));

        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        for selection in selections {
            #[allow(clippy::cast_sign_loss)]
            let Some(node) = self.allocator.get(selection as usize) else {
                continue;
            };
            let Some(element) = node.element() else {
                continue;
            };
            let mut class_list = element.class_list();
            class_list.toggle(name.clone());
        }

        self.effect_document()
    }

    /// Appends the style to the selected elements style list.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    /// When the given property and/or value is invalid.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../spec/manipulate/style.md")]
    pub fn style(&mut self, property: &str, value: &str) -> Result<(), Error<'input>> {
        self.effect_history(&Action::Style {
            property: property.to_string().into(),
            value: value.to_string().into(),
        });

        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        for selection in selections {
            #[allow(clippy::cast_sign_loss)]
            let Some(node) = self.allocator.get(selection as usize) else {
                continue;
            };
            let Some(element) = node.element() else {
                continue;
            };
            if !element.qual_name().is_permitted_attribute(&AttrId::Style) {
                continue;
            }

            let property = self.allocator.alloc_str(property);
            let property = lightningcss::properties::PropertyId::parse_string(property)
                .map_err(|err| Error::ParseError(err.to_string()))?;
            let (value, is_important) = match value.trim_end().split_once("!important") {
                Some((value, "")) => (value, true),
                _ => (value, false),
            };
            let value = self.allocator.alloc_str(value);
            let property = lightningcss::properties::Property::parse_string(
                property,
                value,
                lightningcss::stylesheet::ParserOptions::default(),
            )
            .map_err(|err| Error::ParseError(err.to_string()))?;

            if let Some(mut style) = get_attribute_mut!(element, Style) {
                if is_important {
                    style.0.important_declarations.push(property);
                } else {
                    style.0.declarations.push(property);
                }
            } else {
                let mut style = Style(DeclarationBlock {
                    important_declarations: vec![],
                    declarations: vec![],
                });
                if is_important {
                    style.0.important_declarations.push(property);
                } else {
                    style.0.declarations.push(property);
                }
                set_attribute!(element, Style(style));
            }
        }

        self.effect_document()
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn attr() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.attr("width", "10").unwrap();
                actor.attr("unknown", "foo").unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn class() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.class("my-class").unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn style() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("svg").unwrap();
                actor.style("opacity", "0.5").unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
