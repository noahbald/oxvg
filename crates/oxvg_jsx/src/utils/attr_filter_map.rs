use std::cell::Ref;

use oxvg_collections::{attribute::Attr, element::ElementId};

use crate::{Config, config::State, error::BuildError};

pub fn attr_filter_map<'input, T, F: Fn(&Attr<'input>) -> Result<T, BuildError<'input>>>(
    a: &Ref<'_, Attr<'input>>,
    name: &ElementId<'input>,
    config: &Config,
    state: Option<&State>,
    f: F,
) -> Option<Result<T, BuildError<'input>>> {
    match f(a) {
        Ok(a) => Some(Ok(a)),
        // These errors are okay, just emit warning and drop the attribute.
        Err(BuildError::UnsupportedXMLNS(uri)) => {
            if config.warn() {
                eprintln!(
                    "Warning: dropped `xmlns:{}=\"{uri}\"` namespace from {}",
                    a.name(),
                    if let Some(state) = state {
                        state.component_name.as_str()
                    } else {
                        "document"
                    }
                );
            }
            None
        }
        Err(BuildError::UnknownXMLPrefixAttr(name)) => {
            if config.warn() {
                eprintln!(
                    "Warning: dropped `{name}` attribute from {}",
                    if let Some(state) = state {
                        state.component_name.as_str()
                    } else {
                        "document"
                    }
                );
            }
            None
        }
        Err(BuildError::InvalidJSXName(attr)) => {
            if config.warn() {
                eprintln!(
                    "Warning: dropped `{attr}` attribute from {}. It is not a valid attribute of `{name}`.",
                    if let Some(state) = state {
                        state.component_name.as_str()
                    } else {
                        "document"
                    },
                );
            }
            None
        }
        Err(err) => Some(Err(err)),
    }
}
