use lightningcss::{declaration::DeclarationBlock, traits::Parse};
use oxvg_ast::{get_attribute_mut, set_attribute};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core_attrs::{Integer, Style},
        list_of::{ListOf, SpaceOrComma},
        Attr, AttrId,
    },
};
use oxvg_parse::Parse as _;

use crate::{state::StateElement, utils::get_oxvg_attr, Action, Actor, Error};

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
        self.state.record(
            &Action::Attr {
                name: name.to_string().into(),
                value: value.to_string().into(),
            },
            &self.allocator,
        );
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
        Ok(())
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
        self.state
            .record(&Action::Class(name.clone()), &self.allocator);
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
        Ok(())
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
        self.state.record(
            &Action::Style {
                property: property.to_string().into(),
                value: value.to_string().into(),
            },
            &self.allocator,
        );
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
            };
        }
        Ok(())
    }

    pub(crate) fn get_selections(&mut self) -> Result<Option<Vec<Integer>>, Error<'input>> {
        Ok(self.get_selections_list()?.map(|s| s.list))
    }

    pub(crate) fn get_selections_list(
        &mut self,
    ) -> Result<Option<ListOf<Integer, SpaceOrComma>>, Error<'input>> {
        let selections_element = self.state.get_selections(&self.allocator);
        if let Some(selections) =
            get_oxvg_attr(&selections_element.clone(), StateElement::SELECTION_IDS)?
        {
            let selections = ListOf::<Integer, SpaceOrComma>::parse_string(&selections)
                .map_err(|err| Error::ParseError(err.to_string()))?;
            Ok(Some(selections))
        } else {
            Ok(None)
        }
    }
}
