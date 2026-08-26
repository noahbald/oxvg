use oxvg_ast::{arena::Allocator, element::Element, node::Ref};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        Attr, AttrId,
        core_attrs::Integer,
        list_of::{ListOf, SpaceOrComma},
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
    OXVG_PREFIX, OXVG_XMLNS,
    actions::Action,
    error::Error,
    info::Info,
    utils::{
        assert_oxvg_element, assert_oxvg_xmlns, create_oxvg_attr, create_oxvg_element,
        get_oxvg_attr,
    },
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
            .filter(|e| assert_oxvg_element(*e, StateElement::STATE).is_ok())
            .unwrap_or_else(|| {
                document.create_element(create_oxvg_element(StateElement::STATE), allocator)
            });
        let mut state = Self {
            state: state_element,
            history: None,
            selection: None,
        };

        state_element.remove();
        for element in state_element.children_iter() {
            state.debed_field(element)?;
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
                self.state.remove();
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

    fn debed_field(&mut self, element: Element<'input, 'arena>) -> Result<(), Error<'input>> {
        let name = element.qual_name();
        assert_oxvg_xmlns(name.prefix())?;

        let field = StateElement::try_from(name.local_name().clone())?;
        match field {
            StateElement::History => {
                self.history = Some(element);
            }
            StateElement::Selection => {
                self.selection = Some(element);
            }
        }
        Ok(())
    }

    pub fn record(&mut self, action: &Action<'input>, allocator: &Allocator<'input, 'arena>) {
        let history = self.get_history(allocator);
        action.embed(history, allocator);
    }

    pub fn get_selections(
        &mut self,
        allocator: &Allocator<'input, 'arena>,
    ) -> Element<'input, 'arena> {
        if let Some(e) = self.selection {
            e
        } else {
            let selection = self
                .state
                .as_document()
                .create_element(create_oxvg_element(StateElement::SELECTION), allocator);
            self.state.append_child(*selection);
            self.selection = Some(selection);
            selection
        }
    }

    pub fn get_history(
        &mut self,
        allocator: &Allocator<'input, 'arena>,
    ) -> Element<'input, 'arena> {
        if let Some(e) = &self.history {
            *e
        } else {
            let history = self
                .state
                .as_document()
                .create_element(create_oxvg_element(StateElement::HISTORY), allocator);
            self.state.append_child(*history);
            self.history = Some(history);
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
                .map(Action::from_state)
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
    const CLASS: &'static str = "Class";
    const PATH_INTERSECT: &'static str = "PathIntersect";
    const PATH_UNION: &'static str = "PathUnion";
    const PATH_SUBTRACT: &'static str = "PathSubtract";
    const PATH_XOR: &'static str = "PathXor";
    const STYLE: &'static str = "Style";
    const MATRIX: &'static str = "Matrix";
    const TRANSLATE: &'static str = "Translate";
    const SCALE: &'static str = "Scale";
    const ROTATE: &'static str = "Rotate";
    const SKEW_X: &'static str = "SkewX";
    const SKEW_Y: &'static str = "SkewY";
    const INSERT: &'static str = "Insert";
    const INSERT_NS: &'static str = "InsertNS";
    const DUPLICATE: &'static str = "Duplicate";
    const WRAP: &'static str = "Wrap";
    const CLONE: &'static str = "Clone";
    const ANCHOR_LINK: &'static str = "AnchorLink";
    const GROUP: &'static str = "Group";
    const DELETE: &'static str = "Delete";
    const FLATTEN: &'static str = "Flatten";
    const FORGET: &'static str = "Forget";
    const SELECT: &'static str = "Select";
    const SELECT_MORE: &'static str = "SelectMore";
    const DESELECT: &'static str = "Deselect";

    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn from_state(element: Element<'input, '_>) -> Result<Self, Error<'input>> {
        assert_oxvg_element(element, Self::ACTION)?;

        let Some(id) = get_oxvg_attr(&element, Self::ID)? else {
            return Err(Error::MissingStateAttribute(Self::ID));
        };
        let mut args = element.children_iter().map(|child| {
            assert_oxvg_element(child, Self::ARG)?;
            Ok(child.text_content().unwrap_or_default())
        });
        let n_args = |value: Result<Atom<'input>, Error<'input>>| {
            value.and_then(|value| {
                value
                    .as_str()
                    .parse::<f32>()
                    .map_err(|_| Error::InvalidStateValue {
                        name: Self::MATRIX,
                        value,
                    })
            })
        };

        match id.as_str() {
            Self::ATTR => {
                let Some(name) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(value) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Attr { name, value })
            }
            Self::CLASS => {
                let Some(class) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Class(class))
            }
            Self::PATH_INTERSECT => Ok(Self::PathIntersect),
            Self::PATH_UNION => Ok(Self::PathUnion),
            Self::PATH_SUBTRACT => Ok(Self::PathSubtract),
            Self::PATH_XOR => Ok(Self::PathXor),
            Self::STYLE => {
                let Some(property) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(value) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Style { property, value })
            }
            Self::MATRIX => {
                let mut args = args.map(n_args);
                let Some(a) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(b) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(c) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(d) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(e) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(f) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Matrix(a, b, c, d, e, f))
            }
            Self::TRANSLATE => {
                let mut args = args.map(n_args);
                let Some(x) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let y = args.next().transpose()?;
                Ok(Self::Translate(x, y))
            }
            Self::SCALE => {
                let mut args = args.map(n_args);
                let Some(x) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let y = args.next().transpose()?;
                Ok(Self::Scale(x, y))
            }
            Self::ROTATE => {
                let mut args = args.map(n_args);
                let Some(deg) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let x = args.next().transpose()?;
                let y = args.next().transpose()?;
                if x.is_some() && y.is_none() {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                }
                let origin = x.zip(y);
                Ok(Self::Rotate(deg, origin))
            }
            Self::SKEW_X => {
                let mut args = args.map(n_args);
                let Some(x) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::SkewX(x))
            }
            Self::SKEW_Y => {
                let mut args = args.map(n_args);
                let Some(y) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::SkewY(y))
            }
            Self::INSERT => {
                let Some(name) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Insert(name))
            }
            Self::INSERT_NS => {
                let Some(uri) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                let Some(name) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::InsertNS(uri, name))
            }
            Self::DUPLICATE => Ok(Self::Duplicate),
            Self::WRAP => {
                let Some(name) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Wrap(name))
            }
            Self::CLONE => Ok(Self::Clone),
            Self::ANCHOR_LINK => {
                let Some(href) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::AnchorLink(href))
            }
            Self::GROUP => Ok(Self::Group),
            Self::DELETE => Ok(Self::Delete),
            Self::FLATTEN => Ok(Self::Flatten),
            Self::FORGET => Ok(Self::Forget),
            Self::SELECT => {
                let Some(string) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::Select(string))
            }
            Self::SELECT_MORE => {
                let Some(string) = args.next().transpose()? else {
                    return Err(Error::MissingStateAttribute(Self::ARG));
                };
                Ok(Self::SelectMore(string))
            }
            Self::DESELECT => Ok(Self::Deselect),
            _ => Err(Error::InvalidStateAttribute(id.clone())),
        }
    }

    #[allow(clippy::many_single_char_names)]
    fn embed<'arena>(
        &self,
        parent: Element<'input, 'arena>,
        allocator: &Allocator<'input, 'arena>,
    ) {
        let document = parent.as_document();

        let element = document.create_element(create_oxvg_element(Self::ACTION), allocator);
        element.set_attribute(create_oxvg_attr(Self::ID, self.name().into()));
        parent.append(*element);

        match self {
            Self::Attr {
                name: arg0,
                value: arg1,
            }
            | Self::Style {
                property: arg0,
                value: arg1,
            }
            | Self::InsertNS(arg0, arg1) => {
                Self::embed_arg(element, allocator, arg0.clone());
                Self::embed_arg(element, allocator, arg1.clone());
            }
            Self::Matrix(a, b, c, d, e, f) => {
                Self::embed_arg(element, allocator, a.to_string().into());
                Self::embed_arg(element, allocator, b.to_string().into());
                Self::embed_arg(element, allocator, c.to_string().into());
                Self::embed_arg(element, allocator, d.to_string().into());
                Self::embed_arg(element, allocator, e.to_string().into());
                Self::embed_arg(element, allocator, f.to_string().into());
            }
            Self::Translate(x, y) | Self::Scale(x, y) => {
                Self::embed_arg(element, allocator, x.to_string().into());
                if let Some(y) = y {
                    Self::embed_arg(element, allocator, y.to_string().into());
                }
            }
            Self::Rotate(angle, origin) => {
                Self::embed_arg(element, allocator, angle.to_string().into());
                if let Some((x, y)) = origin {
                    Self::embed_arg(element, allocator, x.to_string().into());
                    Self::embed_arg(element, allocator, y.to_string().into());
                }
            }
            Self::SkewX(arg) | Self::SkewY(arg) => {
                Self::embed_arg(element, allocator, arg.to_string().into());
            }
            Self::Class(arg)
            | Self::Select(arg)
            | Self::SelectMore(arg)
            | Self::Insert(arg)
            | Self::Wrap(arg)
            | Self::AnchorLink(arg) => {
                Self::embed_arg(element, allocator, arg.clone());
            }
            Self::PathIntersect
            | Self::PathUnion
            | Self::PathSubtract
            | Self::PathXor
            | Self::Clone
            | Self::Group
            | Self::Delete
            | Self::Flatten
            | Self::Forget
            | Self::Deselect
            | Self::Duplicate => {}
        }
    }

    fn embed_arg<'arena>(
        parent: Element<'input, 'arena>,
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
            Self::Class(_) => Self::CLASS,
            Self::PathIntersect => Self::PATH_INTERSECT,
            Self::PathUnion => Self::PATH_UNION,
            Self::PathSubtract => Self::PATH_SUBTRACT,
            Self::PathXor => Self::PATH_XOR,
            Self::Style { .. } => Self::STYLE,
            Self::Matrix(..) => Self::MATRIX,
            Self::Translate(..) => Self::TRANSLATE,
            Self::Scale(..) => Self::SCALE,
            Self::Rotate(..) => Self::ROTATE,
            Self::SkewX(_) => Self::SKEW_X,
            Self::SkewY(_) => Self::SKEW_Y,
            Self::Insert(_) => Self::INSERT,
            Self::InsertNS(_, _) => Self::INSERT_NS,
            Self::Duplicate => Self::DUPLICATE,
            Self::Wrap(_) => Self::WRAP,
            Self::Clone => Self::CLONE,
            Self::AnchorLink(_) => Self::ANCHOR_LINK,
            Self::Group => Self::GROUP,
            Self::Delete => Self::DELETE,
            Self::Flatten => Self::FLATTEN,
            Self::Forget => Self::FORGET,
            Self::Select(_) => Self::SELECT,
            Self::SelectMore(_) => Self::SELECT_MORE,
            Self::Deselect => Self::DESELECT,
        }
    }

    #[cfg(feature = "napi")]
    #[allow(clippy::many_single_char_names)]
    /// Converts to a napi-compatible type
    pub fn to_napi(&self) -> ActionNapi {
        match self {
            Self::Attr { name, value } => ActionNapi::Attr {
                name: name.to_string(),
                value: value.to_string(),
            },
            Self::Class(name) => ActionNapi::Class(name.to_string()),
            Self::PathIntersect => ActionNapi::PathIntersect,
            Self::PathUnion => ActionNapi::PathUnion,
            Self::PathSubtract => ActionNapi::PathSubtract,
            Self::PathXor => ActionNapi::PathXor,
            Self::Style { property, value } => ActionNapi::Style {
                property: property.to_string(),
                value: value.to_string(),
            },
            Self::Matrix(a, b, c, d, e, f) => ActionNapi::Matrix(
                *a as f64, *b as f64, *c as f64, *d as f64, *e as f64, *f as f64,
            ),
            Self::Translate(x, y) => ActionNapi::Translate(*x as f64, y.map(|y| y as f64)),
            Self::Scale(x, y) => ActionNapi::Scale(*x as f64, y.map(|y| y as f64)),
            Self::Rotate(angle, origin) => {
                ActionNapi::Rotate(*angle as f64, origin.map(|(x, y)| (x as f64, y as f64)))
            }
            Self::SkewX(x) => ActionNapi::SkewX(*x as f64),
            Self::SkewY(y) => ActionNapi::SkewY(*y as f64),
            Self::InsertNS(uri, name) => ActionNapi::InsertNS(uri.to_string(), name.to_string()),
            Self::Insert(name) => ActionNapi::Insert(name.to_string()),
            Self::Duplicate => ActionNapi::Duplicate,
            Self::Wrap(name) => ActionNapi::Wrap(name.to_string()),
            Self::Clone => ActionNapi::Clone,
            Self::AnchorLink(href) => ActionNapi::AnchorLink(href.to_string()),
            Self::Group => ActionNapi::Group,
            Self::Delete => ActionNapi::Delete,
            Self::Flatten => ActionNapi::Flatten,
            Self::Forget => ActionNapi::Forget,
            Self::Select(query) => ActionNapi::Select(query.to_string()),
            Self::SelectMore(query) => ActionNapi::SelectMore(query.to_string()),
            Self::Deselect => ActionNapi::Deselect,
        }
    }

    #[cfg(feature = "napi")]
    #[allow(clippy::many_single_char_names)]
    /// Converts to a napi-compatible type
    pub fn from_napi(other: ActionNapi) -> Action<'static> {
        match other {
            ActionNapi::Attr { name, value } => Action::Attr {
                name: name.into(),
                value: value.into(),
            },
            ActionNapi::Class(name) => Action::Class(name.into()),
            ActionNapi::PathIntersect => Action::PathIntersect,
            ActionNapi::PathUnion => Action::PathUnion,
            ActionNapi::PathSubtract => Action::PathSubtract,
            ActionNapi::PathXor => Action::PathXor,
            ActionNapi::Style { property, value } => Action::Style {
                property: property.into(),
                value: value.into(),
            },
            ActionNapi::Matrix(a, b, c, d, e, f) => {
                Action::Matrix(a as f32, b as f32, c as f32, d as f32, e as f32, f as f32)
            }
            ActionNapi::Translate(x, y) => Action::Translate(x as f32, y.map(|y| y as f32)),
            ActionNapi::Scale(x, y) => Action::Scale(x as f32, y.map(|y| y as f32)),
            ActionNapi::Rotate(angle, origin) => {
                Action::Rotate(angle as f32, origin.map(|(x, y)| (x as f32, y as f32)))
            }
            ActionNapi::SkewX(x) => Action::SkewX(x as f32),
            ActionNapi::SkewY(y) => Action::SkewY(y as f32),
            ActionNapi::Insert(name) => Action::Insert(name.into()),
            ActionNapi::InsertNS(uri, name) => Action::InsertNS(uri.into(), name.into()),
            ActionNapi::Duplicate => Action::Duplicate,
            ActionNapi::Wrap(name) => Action::Wrap(name.into()),
            ActionNapi::Clone => Action::Clone,
            ActionNapi::AnchorLink(href) => Action::AnchorLink(href.into()),
            ActionNapi::Group => Action::Group,
            ActionNapi::Delete => Action::Delete,
            ActionNapi::Flatten => Action::Flatten,
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
