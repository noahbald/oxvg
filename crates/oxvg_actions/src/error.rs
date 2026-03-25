use oxvg_collections::atom::Atom;

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(from_wasm_abi, into_wasm_abi))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// Errors that may be emitted by [`crate::Actor`].
pub enum Error<'input> {
    /// The document had no elements that could be acted upon
    NoRootElement,
    /// The document could not be parsed
    ParseError(String),
    /// The document failed to serialize
    SerializeError(String),
    /// An `oxvg:state` element or attribute has a non-oxvg xmlns
    InvalidStateXMLNS,
    /// An `oxvg` prefixed element is unknown or invalid
    InvalidStateElement(Atom<'input>),
    /// An `oxvg` prefixed attribute is unknown or invalid
    InvalidStateAttribute(Atom<'input>),
    /// An expected `oxvg` prefixed attribute is missing
    MissingStateAttribute(&'static str),
    /// An expected `oxvg` prefixed element is missing
    MissingStateElement(&'static str),
    /// An `oxvg` prefixed attribute has an invalid value
    InvalidStateValue {
        /// The name of the attribute
        name: &'static str,
        /// The value of the attribute
        value: Atom<'input>,
    },
    /// The `select` action was called with an invalid selector
    InvalidSelector(String),
}

impl std::error::Error for Error<'_> {}

const WITHIN_STATE: &str = "within `oxvg:state` element.";
impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRootElement => f.write_str("Document has no root element"),
            Self::ParseError(err) => f.write_fmt(format_args!("Could not parse item: {err}")),
            Self::SerializeError(err) => {
                f.write_fmt(format_args!("Could not serialize item: {err}"))
            }
            Self::InvalidStateXMLNS => f.write_fmt(format_args!("Unexpected xmlns {WITHIN_STATE}")),
            Self::InvalidStateElement(name) => {
                f.write_fmt(format_args!("Unexpected element `{name}` {WITHIN_STATE}"))
            }
            Self::InvalidStateAttribute(name) => {
                f.write_fmt(format_args!("Unexpected attribute `{name}` {WITHIN_STATE}"))
            }
            Self::MissingStateAttribute(name) => {
                f.write_fmt(format_args!("Missing `{name}` attribute {WITHIN_STATE}"))
            }
            Self::MissingStateElement(name) => {
                f.write_fmt(format_args!("Missing `{name}` element {WITHIN_STATE}"))
            }
            Self::InvalidStateValue { name, value } => f.write_fmt(format_args!(
                "Invalid value `{value}` on `{name}` found {WITHIN_STATE}"
            )),
            Self::InvalidSelector(query) => f.write_fmt(format_args!(
                "Invalid or unsupported query selector given: `{query}`"
            )),
        }
    }
}
