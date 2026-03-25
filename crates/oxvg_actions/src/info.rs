use oxvg_ast::{
    arena::Allocator,
    node::{self, Node},
};

use oxvg_collections::{
    atom::Atom,
    attribute::{AttrId, AttributeGroup},
    element::{ElementCategory, ElementId},
};
use oxvg_serialize::{PrinterOptions, ToValue as _};
#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::Error;

#[cfg(not(test))]
type Set<T> = std::collections::HashSet<T>;
#[cfg(not(test))]
type Map<K, V> = std::collections::HashMap<K, V>;
#[cfg(test)]
type Set<T> = std::collections::BTreeSet<T>;
#[cfg(test)]
type Map<K, V> = std::collections::BTreeMap<K, V>;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug)]
/// The information shared by elements matching the elements in `oxvg:selection`
pub struct Info<'input> {
    /// The element id, if all elements are of the same id
    pub element_id: Option<ElementId<'input>>,
    /// The content-model common to the selected elements
    pub content_model: ContentModel,
    /// The attributes common to the selected elements
    pub attributes: AttrModel<'input>,
    /// The text content common to the selected elements
    pub text: Option<Atom<'input>>,
}

#[cfg(feature = "napi")]
#[napi(object)]
/// The information shared by elements matching the elements in `oxvg:selection`
pub struct InfoNapi {
    /// The element id, if all elements are of the same id
    pub element_id: Option<oxvg_collections::element::ElementIdNapi>,
    /// The content-model common to the selected elements
    pub content_model: ContentModelNapi,
    /// The attributes common to the selected elements
    pub attributes: AttrModelNapi,
    /// The text content common to the selected elements
    pub text: Option<String>,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug)]
/// The content-model common to the selected elements
pub struct ContentModel {
    /// The element categories permitted by all selected elements
    pub permitted_categories: ElementCategory,
    /// The elements permitted by all selected elements
    pub permitted_elements: Set<&'static ElementId<'static>>,
}

impl ContentModel {
    #[cfg(feature = "napi")]
    fn to_napi(&self) -> ContentModelNapi {
        ContentModelNapi {
            permitted_categories: self.permitted_categories.bits(),
            permitted_elements: self
                .permitted_elements
                .iter()
                .map(std::ops::Deref::deref)
                .map(ElementId::to_napi)
                .collect(),
        }
    }
}

#[cfg(feature = "napi")]
#[napi(object)]
/// The content-model common to the selected elements
pub struct ContentModelNapi {
    /// The element categories permitted by all selected elements
    pub permitted_categories: u32,
    /// The elements permitted by all selected elements
    pub permitted_elements: Vec<oxvg_collections::element::ElementIdNapi>,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug)]
/// The attributes common to the selected elements
pub struct AttrModel<'input> {
    /// The expected attribute groups shared by all selected elements
    pub expected_attribute_groups: AttributeGroup,
    /// The expected attributes shared by all selected elements
    pub expected_attributes: Set<&'static AttrId<'static>>,
    /// The attribute values matching between all selected elements
    pub values: Map<AttrId<'input>, Atom<'static>>,
}

impl AttrModel<'_> {
    #[cfg(feature = "napi")]
    fn to_napi(&self) -> AttrModelNapi {
        AttrModelNapi {
            expected_attribute_groups: self.expected_attribute_groups.bits(),
            expected_attributes: self
                .expected_attributes
                .iter()
                .map(std::ops::Deref::deref)
                .map(AttrId::to_napi)
                .collect(),
            values: self
                .values
                .iter()
                .map(|(id, value)| (id.to_napi(), value.to_string()))
                .collect(),
        }
    }
}

#[cfg(feature = "napi")]
#[napi(object)]
/// The attributes common to the selected elements
pub struct AttrModelNapi {
    /// The expected attribute groups shared by all selected elements
    pub expected_attribute_groups: u32,
    /// The expected attributes shared by all selected elements
    pub expected_attributes: Vec<oxvg_collections::attribute::AttrIdNapi>,
    /// The attribute values matching between all selected elements
    pub values: Vec<(oxvg_collections::attribute::AttrIdNapi, String)>,
}

impl<'input> Info<'input> {
    pub(crate) fn new(
        selections: &[usize],
        allocator: &Allocator<'input, '_>,
    ) -> Result<Option<Self>, Error<'input>> {
        let selected_elements = selections
            .iter()
            .filter_map(|id| allocator.get(*id))
            .map(Node::element);
        if selected_elements.clone().any(|element| element.is_none()) {
            return Ok(None);
        }

        let mut text = Some(Atom::Static(""));
        let mut element_ids = Set::new();
        let mut permitted_categories = ElementCategory::all();
        let mut permitted_elements = Set::new();
        let mut expected_attribute_groups = AttributeGroup::all();
        let mut expected_attributes = Set::new();
        let mut values = Map::new();
        for (i, element) in selected_elements.flatten().enumerate() {
            let name = element.qual_name().clone();

            let map: Result<Map<_, _>, _> = element
                .attributes()
                .into_iter()
                .filter(|attr| {
                    if i > 0 {
                        values.contains_key(attr.name())
                    } else {
                        true
                    }
                })
                .map(|attr| {
                    attr.to_value_string(PrinterOptions::default())
                        .map(|a| (attr.name().clone(), Atom::from(a)))
                        .map_err(|err| Error::SerializeError(err.to_string()))
                })
                .filter(|result| match result {
                    Ok((name, value)) if i > 0 => {
                        values.get(name).expect("value was filtered") == value
                    }
                    _ => true,
                })
                .collect();
            values = map?;
            if let Some(inner) = text.as_mut() {
                if inner.as_str() == ""
                    && element
                        .child_nodes_iter()
                        .all(|n| n.node_type() == node::Type::Text)
                {
                    *inner = element.text_content().unwrap_or_default();
                } else {
                    text = None;
                }
            }
            permitted_categories = permitted_categories.union(name.permitted_categories());
            permitted_elements = name
                .permitted_elements()
                .iter()
                .filter(|name| {
                    if i > 0 {
                        permitted_elements.contains(name)
                    } else {
                        true
                    }
                })
                .collect();
            expected_attribute_groups =
                expected_attribute_groups.union(name.expected_attribute_groups());
            expected_attributes = name
                .expected_attributes()
                .iter()
                .filter(|name| {
                    if i > 0 {
                        expected_attributes.contains(name)
                    } else {
                        true
                    }
                })
                .collect();
            element_ids.insert(name);
        }

        if element_ids.is_empty() {
            permitted_categories = ElementCategory::empty();
            expected_attribute_groups = AttributeGroup::empty();
        }
        let element_id = if element_ids.len() == 1 {
            element_ids.into_iter().next()
        } else {
            None
        };
        Ok(Some(Self {
            element_id,
            content_model: ContentModel {
                permitted_categories,
                permitted_elements,
            },
            attributes: AttrModel {
                expected_attribute_groups,
                expected_attributes,
                values,
            },
            text,
        }))
    }

    #[cfg(feature = "napi")]
    /// Converts to a napi-compatible type
    pub fn to_napi(&self) -> InfoNapi {
        InfoNapi {
            element_id: self.element_id.as_ref().map(ElementId::to_napi),
            content_model: self.content_model.to_napi(),
            attributes: self.attributes.to_napi(),
            text: self.text.as_ref().map(Atom::to_string),
        }
    }
}
