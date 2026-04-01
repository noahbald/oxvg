use oxvg_ast::{arena::Allocator, element::Element, node::Ref};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core_attrs::Integer,
        list_of::{ListOf, SpaceOrComma},
        Attr, AttrId,
    },
    name::{Prefix, QualName},
};
use oxvg_parse::Parse;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg(feature = "napi")]
use crate::actions::ActionNapi;
#[cfg(feature = "napi")]
use crate::info::InfoNapi;

use crate::{
    actions::Action,
    error::Error,
    info::Info,
    utils::{
        assert_oxvg_element, assert_oxvg_xmlns, create_oxvg_attr, create_oxvg_element,
        get_oxvg_attr,
    },
    OXVG_PREFIX, OXVG_XMLNS,
};

#[allow(clippy::struct_field_names)]
pub(crate) struct State<'input, 'arena> {
    pub state: Element<'input, 'arena>,
    pub history: Option<Element<'input, 'arena>>,
    // TODO: pub ui: Vec<UIAction>,
    pub selection: Option<Element<'input, 'arena>>,
    // TODO: pub clipboard: Option<Element<'input, 'arena>>,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug)]
/// An information rich struct based on information derived from the `oxvg:state`
/// element.
pub struct DerivedState<'input> {
    /// The list of actions specified by `oxvg:history`
    pub history: Vec<Action<'input>>,
    /// The ids specified by `oxvg:selection`
    pub selection: Vec<usize>,
    /// The information shared by elements matching the elements in `oxvg:selection`
    pub info: Option<Info<'input>>,
    // TODO: issues: Vec<Issue>,
}

#[cfg(feature = "napi")]
#[napi(object)]
/// See [`DerivedState`]
pub struct DerivedStateNapi {
    /// The list of actions specified by `oxvg:history`
    pub history: Vec<ActionNapi>,
    /// The ids specified by `oxvg:selection`
    pub selection: Vec<u32>,
    /// The information shared by elements matching the elements in `oxvg:selection`
    pub info: Option<InfoNapi>,
    // TODO: issues: Vec<Issue>,
}

pub(crate) enum StateElement {
    History,
    Selection,
}

impl<'input, 'arena> State<'input, 'arena> {
    /// Creates a state from the given document by removing the `oxvg:state` element and using it's data
    pub fn debed(
        root: Ref<'input, 'arena>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Result<Self, Error<'input>> {
        let Some(element) = root.find_element() else {
            return Err(Error::NoRootElement);
        };
        let document = element.as_document();
        let state_element = element
            .last_element_child()
            .and_then(|e| {
                if assert_oxvg_element(&e, StateElement::STATE).is_ok() {
                    Some(e)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                document.create_element(create_oxvg_element(StateElement::STATE), allocator)
            });
        let mut state = Self {
            state: state_element.clone(),
            history: None,
            selection: None,
        };

        state_element.remove();
        for element in state_element.children_iter() {
            state.debed_field(&element)?;
        }

        Ok(state)
    }

    pub fn embed(&mut self, root: Ref<'input, 'arena>) -> Result<(), Error<'input>> {
        if self.history.is_none() && self.selection.is_none() {
            return Ok(());
        }

        let Some(element) = root.find_element() else {
            return Err(Error::NoRootElement);
        };
        if let Some(maybe_this) = element.last_element_child() {
            if maybe_this != self.state {
                element.set_attribute(Attr::Unparsed {
                    attr_id: AttrId::Unknown(QualName {
                        prefix: Prefix::XMLNS,
                        local: OXVG_PREFIX.into(),
                    }),
                    value: OXVG_XMLNS.into(),
                });
                element.append_child(*self.state);
            }
        } else {
            element.set_attribute(Attr::Unparsed {
                attr_id: AttrId::Unknown(QualName {
                    prefix: Prefix::XMLNS,
                    local: OXVG_PREFIX.into(),
                }),
                value: OXVG_XMLNS.into(),
            });
            element.append_child(*self.state);
        }
        Ok(())
    }

    fn debed_field(&mut self, element: &Element<'input, 'arena>) -> Result<(), Error<'input>> {
        let name = element.qual_name();
        assert_oxvg_xmlns(name.prefix())?;

        let field = StateElement::try_from(name.local_name().clone())?;
        match field {
            StateElement::History => {
                self.history = Some(element.clone());
            }
            StateElement::Selection => {
                self.selection = Some(element.clone());
            }
        }
        Ok(())
    }

    pub fn record(&mut self, action: &Action<'input>, allocator: &Allocator<'input, 'arena>) {
        let history = self.get_history(allocator);
        action.embed(&history, allocator);
    }

    pub fn get_selections(
        &mut self,
        allocator: &Allocator<'input, 'arena>,
    ) -> Element<'input, 'arena> {
        if let Some(e) = &self.selection {
            e.clone()
        } else {
            let selection = self
                .state
                .as_document()
                .create_element(create_oxvg_element(StateElement::SELECTION), allocator);
            self.state.append_child(*selection);
            self.selection = Some(selection.clone());
            selection
        }
    }

    pub fn get_history(
        &mut self,
        allocator: &Allocator<'input, 'arena>,
    ) -> Element<'input, 'arena> {
        if let Some(e) = &self.history {
            e.clone()
        } else {
            let history = self
                .state
                .as_document()
                .create_element(create_oxvg_element(StateElement::HISTORY), allocator);
            self.state.append_child(*history);
            self.history = Some(history.clone());
            history
        }
    }
}

impl<'input, 'arena> DerivedState<'input> {
    pub(crate) fn from_state(
        state: &State<'input, 'arena>,
        allocator: &Allocator<'input, 'arena>,
    ) -> Result<Self, Error<'input>> {
        let selection = match &state.selection {
            Some(e) => {
                if let Some(value) = get_oxvg_attr(e, StateElement::SELECTION_IDS)? {
                    let list = ListOf::<Integer, SpaceOrComma>::parse_string(value.as_str())
                        .map_err(|err| Error::ParseError(err.to_string()))?;
                    #[allow(clippy::cast_sign_loss)]
                    list.list.into_iter().map(|n| n as usize).collect()
                } else {
                    vec![]
                }
            }
            None => vec![],
        };
        Ok(Self {
            history: state
                .history
                .iter()
                .flat_map(Element::children_iter)
                .map(|e| Action::from_state(&e))
                .collect::<Result<Vec<_>, _>>()?,
            info: Info::new(&selection, allocator)?,
            selection,
        })
    }

    #[cfg(feature = "napi")]
    /// Converts to a napi-compatible type
    pub fn to_napi(&self) -> DerivedStateNapi {
        DerivedStateNapi {
            history: self.history.iter().map(Action::to_napi).collect(),
            selection: self.selection.iter().map(|n| *n as u32).collect(),
            info: self.info.as_ref().map(Info::to_napi),
        }
    }
}

impl<'input> Action<'input> {
    // OXVG Elements
    const ACTION: &'static str = "action";
    // OXVG Attrs
    const ARG: &'static str = "arg";
    const ID: &'static str = "id";
    // Members
    const ATTR: &'static str = "Attr";
    const FORGET: &'static str = "Forget";
    const SELECT: &'static str = "Select";
    const SELECT_MORE: &'static str = "SelectMore";
    const DESELECT: &'static str = "Deselect";

    fn from_state(element: &Element<'input, '_>) -> Result<Self, Error<'input>> {
        assert_oxvg_element(element, Self::ACTION)?;

        let Some(id) = get_oxvg_attr(element, Self::ID)? else {
            return Err(Error::MissingStateAttribute(Self::ID));
        };
        let mut args = element.children_iter().map(|child| {
            assert_oxvg_element(&child, Self::ARG)?;
            Ok(child.text_content().unwrap_or_default())
        });

        match id.as_str() {
            Self::SELECT => {
                let Some(string) = args.next() else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Select(string?))
            }
            _ => Err(Error::InvalidStateAttribute(id.clone())),
        }
    }

    fn embed<'arena>(
        &self,
        parent: &Element<'input, 'arena>,
        allocator: &Allocator<'input, 'arena>,
    ) {
        let document = parent.as_document();

        let element = document.create_element(create_oxvg_element(Self::ACTION), allocator);
        element.set_attribute(create_oxvg_attr(Self::ID, self.name().into()));
        parent.append(*element);

        match self {
            Self::Attr { name, value } => {
                Self::embed_arg(&element, allocator, name.clone());
                Self::embed_arg(&element, allocator, value.clone());
            }
            Self::Select(query) | Self::SelectMore(query) => {
                Self::embed_arg(&element, allocator, query.clone());
            }
            Self::Forget | Self::Deselect => {}
        }
    }

    fn embed_arg<'arena>(
        parent: &Element<'input, 'arena>,
        allocator: &Allocator<'input, 'arena>,
        arg_atom: Atom<'input>,
    ) {
        let document = parent.as_document();
        let arg = document.create_element(create_oxvg_element(Self::ARG), allocator);
        arg.set_text_content(arg_atom, allocator);
        parent.append(*arg);
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Attr { .. } => Self::ATTR,
            Self::Forget => Self::FORGET,
            Self::Select(_) => Self::SELECT,
            Self::SelectMore(_) => Self::SELECT_MORE,
            Self::Deselect => Self::DESELECT,
        }
    }

    #[cfg(feature = "napi")]
    /// Converts to a napi-compatible type
    pub fn to_napi(&self) -> ActionNapi {
        match self {
            Self::Attr { name, value } => ActionNapi::Attr {
                name: name.to_string(),
                value: value.to_string(),
            },
            Self::Forget => ActionNapi::Forget,
            Self::Select(query) => ActionNapi::Select(query.to_string()),
            Self::SelectMore(query) => ActionNapi::SelectMore(query.to_string()),
            Self::Deselect => ActionNapi::Deselect,
        }
    }

    #[cfg(feature = "napi")]
    /// Converts to a napi-compatible type
    pub fn from_napi(other: ActionNapi) -> Action<'static> {
        match other {
            ActionNapi::Attr { name, value } => Action::Attr {
                name: name.into(),
                value: value.into(),
            },
            ActionNapi::Forget => Action::Forget,
            ActionNapi::Select(query) => Action::Select(query.into()),
            ActionNapi::SelectMore(query) => Action::SelectMore(query.into()),
            ActionNapi::Deselect => Action::Deselect,
        }
    }
}

impl StateElement {
    pub const STATE: &'static str = "state";
    pub const HISTORY: &'static str = "history";
    pub const SELECTION: &'static str = "selection";
    pub const SELECTION_IDS: &'static str = "ids";

    pub fn _as_str(&self) -> &'static str {
        match self {
            Self::History => Self::HISTORY,
            Self::Selection => Self::SELECTION,
        }
    }
}

impl<'input> TryFrom<Atom<'input>> for StateElement {
    type Error = Error<'input>;

    fn try_from(value: Atom<'input>) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            Self::HISTORY => Self::History,
            Self::SELECTION => Self::Selection,
            _ => return Err(Error::InvalidStateElement(value)),
        })
    }
}
