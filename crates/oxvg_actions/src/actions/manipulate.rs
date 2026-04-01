use oxvg_collections::attribute::{
    core_attrs::Integer,
    list_of::{ListOf, SpaceOrComma},
    Attr, AttrId,
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

    fn get_selections(&mut self) -> Result<Option<Vec<Integer>>, Error<'input>> {
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
