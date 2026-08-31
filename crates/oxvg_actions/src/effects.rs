//! Effects are required to effectively commit action changes to the document.
//!
//! Some code style rules regarding effects:
//! - Effect should be called only for action methods, not in utility functions used by actions.
//! - Effects should called when all relevant actions make an effect, as specified in `./spec/`.
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core_attrs::Integer,
        list_of::{ListOf, SpaceOrComma},
    },
};
use oxvg_serialize::{PrinterOptions, ToValue as _};

use crate::{
    Action, Actor, Error,
    state::StateElement,
    utils::{create_oxvg_attr, create_oxvg_attr_id, to_id},
};

#[derive(Clone, Copy)]
pub(crate) enum StateEffect {
    Remove,
    Embed,
}

impl<'input> Actor<'input, '_> {
    /// Should be called for direct state manipulation
    pub(crate) fn effect_state(&mut self, effect: StateEffect) -> Result<(), Error<'input>> {
        match effect {
            StateEffect::Remove => {
                self.state.state.remove();
                Ok(())
            }
            StateEffect::Embed => self.state.embed(self.root),
        }
    }

    /// Must be called at the start of an action that affects history.
    pub(crate) fn effect_history(&mut self, action: &Action<'input>) {
        self.state.record(action, &self.allocator);
    }

    // TODO: pub fn clipboard

    /// Must be the last call of an action that affects the document. Doesn't
    /// need to be called for early returns prior to affecting the document.
    ///
    /// This is a no-op, but may run assertions or other operations in future.
    #[allow(clippy::unnecessary_wraps)]
    #[allow(clippy::unused_self)]
    #[inline]
    pub(crate) fn effect_document(&self) -> Result<(), Error<'static>> {
        Ok(())
    }

    /// Must be called at the end of an action that affects selection.
    pub(crate) fn effect_selection(
        &mut self,
        selections: &ListOf<Integer, SpaceOrComma>,
    ) -> Result<(), Error<'input>> {
        let state_selections = self.state.get_selections(&self.allocator);
        if selections.list.is_empty() {
            state_selections.remove_attribute(&create_oxvg_attr_id(StateElement::SELECTION_IDS));
            state_selections.remove();
            return Ok(());
        }

        let selections: Atom = selections
            .to_value_string(PrinterOptions::default())
            .map_err(|err| Error::SerializeError(err.to_string()))?
            .into();
        state_selections.set_attribute(create_oxvg_attr(StateElement::SELECTION_IDS, selections));
        self.effect_state(StateEffect::Embed)
    }

    /// Must be called at the end of an action. For an action that affects selection, must
    /// be called after [`Actor::effect_selection`].
    pub(crate) fn effect_tree(&mut self) -> Result<(), Error<'input>> {
        if let Some(root) = self.root.find_element() {
            self.state.state.remove();
            root.append_child(*self.state.state);
        }
        let Some(selection) = self.get_selections()? else {
            self.allocator.reorder(self.root);
            return Ok(());
        };

        let selections: Vec<_> = self.get_selection_nodes(Some(selection)).collect();
        self.allocator.reorder(self.root);
        self.effect_selection(&selections.into_iter().map(to_id).collect())
    }
}
