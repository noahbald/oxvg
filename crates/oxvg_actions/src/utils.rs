use std::cell;

use oxvg_ast::element::Element;
use oxvg_collections::{
    atom::Atom,
    attribute::{Attr, AttrId},
    element::ElementId,
    name::{Prefix, QualName, NS},
};

use crate::{error::Error, OXVG_PREFIX, OXVG_XMLNS};

pub fn is_oxvg_xmlns(prefix: &Prefix) -> bool {
    matches!(
            prefix,
            Prefix::Unknown {
                ns: NS::Unknown(ns),
                ..
            } if ns.as_str() == OXVG_XMLNS)
}

pub fn assert_oxvg_xmlns(prefix: &Prefix) -> Result<(), Error<'static>> {
    if is_oxvg_xmlns(prefix) {
        Ok(())
    } else {
        Err(Error::InvalidStateXMLNS)
    }
}

pub fn assert_oxvg_element<'input>(
    element: &Element<'input, '_>,
    local_name: &str,
) -> Result<(), Error<'input>> {
    let name = element.qual_name();
    assert_oxvg_xmlns(name.prefix())?;
    if name.local_name().as_str() == local_name {
        Ok(())
    } else {
        Err(Error::InvalidStateElement(name.local_name().clone()))
    }
}

pub fn get_oxvg_attr<'a, 'input>(
    element: &'a Element<'input, '_>,
    local_name: &'static str,
) -> Result<Option<cell::Ref<'a, Atom<'input>>>, Error<'input>> {
    let attr = element
        .attributes()
        .get_named_item(&AttrId::Unknown(create_oxvg_qual_name(local_name)));
    let Some(attr) = attr.as_ref() else {
        return Ok(None);
    };
    assert_oxvg_xmlns(attr.prefix())?;
    let value = cell::Ref::filter_map(cell::Ref::clone(attr), |attr| match attr {
        Attr::Unparsed { value, .. } => Some(value),
        _ => unreachable!(),
    })
    .map_err(|_| Error::InvalidStateAttribute(Atom::Static(local_name)))?;
    Ok(Some(value))
}

pub const fn create_oxvg_element(local_name: &'static str) -> ElementId<'static> {
    ElementId::Unknown(create_oxvg_qual_name(local_name))
}

pub const fn create_oxvg_attr<'input>(
    local_name: &'static str,
    value: Atom<'input>,
) -> Attr<'input> {
    Attr::Unparsed {
        attr_id: AttrId::Unknown(create_oxvg_qual_name(local_name)),
        value,
    }
}

pub const fn create_oxvg_qual_name(local_name: &'static str) -> QualName<'static> {
    QualName {
        prefix: Prefix::Unknown {
            prefix: Some(Atom::Static(OXVG_PREFIX)),
            ns: NS::Unknown(Atom::Static(OXVG_XMLNS)),
        },
        local: Atom::Static(local_name),
    }
}
