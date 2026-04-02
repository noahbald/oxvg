//! WASM bindings for OXVG
extern crate console_error_panic_hook;
use oxvg_ast::{arena::Allocator, serialize::Node as _, visitor::Info, xmlwriter::Options};
use oxvg_collections::{attribute::AttributeGroup, element::ElementCategory};
use oxvg_optimiser::{Extends, Jobs};

use wasm_bindgen::prelude::*;

type Atom = oxvg_collections::atom::Atom<'static>;
type ElementId = oxvg_collections::element::ElementId<'static>;
type AttrId = oxvg_collections::attribute::AttrId<'static>;
type Prefix = oxvg_collections::name::Prefix<'static>;

#[cfg(not(feature = "web_sys"))]
use oxvg_ast::parse::roxmltree::{parse_tree_with_allocator, parse_with_options, ParsingOptions};
#[cfg(feature = "web_sys")]
use oxvg_ast::parse::web_sys::{parse, parse_tree_with_allocator, Document};

#[wasm_bindgen]
/// Optimise an SVG document using the provided config
///
/// # Errors
/// - If the document fails to parse
/// - If any of the optimisations fail
/// - If the optimised document fails to serialize
///
/// # Examples
///
/// Optimise svg with the default configuration
///
/// ```js
/// import { optimise } from "@oxvg/wasm";
///
/// const result = optimise(`<svg />`);
/// ```
///
/// Or, provide your own config
///
/// ```js
/// import { optimise } from "@oxvg/wasm";
///
/// // Only optimise path data
/// const result = optimise(`<svg />`, { convertPathData: {} });
/// ```
///
/// Or, extend a preset
///
/// ```js
/// import { optimise, extend } from "@oxvg/wasm";
///
/// const result = optimise(
///     `<svg />`,
///     extend("default", { convertPathData: { removeUseless: false } }),
/// );
/// ```
pub fn optimise(svg: &str, config: Option<Jobs>) -> Result<String, String> {
    console_error_panic_hook::set_once();

    let config = config.unwrap_or_default();
    #[cfg(feature = "roxmltree")]
    return parse_with_options(
        svg,
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
        |dom, allocator| {
            config
                .run(dom, &Info::new(allocator))
                .map_err(|e| e.to_string())?;
            dom.serialize().map_err(|e| e.to_string())
        },
    )
    .map_err(|e| e.to_string())?;
    #[cfg(feature = "web_sys")]
    return parse(svg, |dom, allocator| {
        config
            .run(dom, &Info::new(allocator))
            .map_err(|e| e.to_string())?;
        dom.serialize().map_err(|e| e.to_string())
    })
    .map_err(|e| e.to_string())?;
}

#[wasm_bindgen(js_name = convertSvgoConfig)]
/// Converts a JSON value of SVGO's `Config["plugins"]` into [`Jobs`].
///
/// Note that this will deduplicate any plugins listed.
///
/// # Errors
///
/// If a config file cannot be deserialized into jobs. This may fail even if
/// the config is valid for SVGO, such as if
///
/// - The config contains custom plugins
/// - The plugin parameters are incompatible with OXVG
/// - The underlying deserialization process fails
///
/// If you believe an errors should be fixed, please raise an issue
/// [here](https://github.com/noahbald/oxvg/issues)
pub fn convert_svgo_config(config: Option<Vec<JsValue>>) -> Result<Jobs, String> {
    if let Some(config) = config {
        let config = config
            .into_iter()
            .map(serde_wasm_bindgen::from_value::<serde_json::Value>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string());
        Jobs::from_svgo_plugin_config(Some(config?)).map_err(|err| err.to_string())
    } else {
        Jobs::from_svgo_plugin_config(None).map_err(|err| err.to_string())
    }
}

#[wasm_bindgen]
#[allow(clippy::needless_pass_by_value)]
/// Returns the given config with omitted options replaced with the config provided by `extends`.
/// I.e. acts like `{ ...extends, ...config }`
pub fn extend(extends: &Extends, config: Option<Jobs>) -> Jobs {
    match config {
        Some(ref jobs) => extends.extend(jobs),
        None => extends.jobs(),
    }
}

/// Converts an atom to a plain string
#[wasm_bindgen(js_name = atomToJS)]
pub fn atom_to_js(atom: &Atom) -> String {
    atom.to_string()
}
/// Converts a plain string to an atom
#[wasm_bindgen(js_name = jsToAtom)]
pub fn js_to_atom(string: String) -> Atom {
    string.into()
}

#[wasm_bindgen(js_name = elementPrefix)]
/// Returns the prefix of the qualified name.
pub fn element_prefix(element: &ElementId) -> Prefix {
    element.prefix().clone()
}
#[wasm_bindgen(js_name = elementLocalName)]
/// Returns the local part of the qualified name.
pub fn element_local_name(element: &ElementId) -> Atom {
    element.local_name().clone()
}
#[wasm_bindgen(js_name = attrPrefix)]
/// Returns the prefix of the qualified name.
pub fn attr_prefix(attr: &AttrId) -> Prefix {
    attr.prefix().clone()
}
#[wasm_bindgen(js_name = attrLocalName)]
/// Returns the local part of the qualified name.
pub fn attr_local_name(attr: &AttrId) -> Atom {
    attr.local_name().clone()
}
#[wasm_bindgen(js_name = prefixNS)]
/// Returns the URI of the prefix.
pub fn prefix_ns(prefix: &Prefix) -> Atom {
    prefix.ns().uri().clone()
}
#[wasm_bindgen(js_name = prefixAlias)]
/// Returns the alias of the prefix, if any.
pub fn prefix_alias(prefix: &Prefix) -> Option<Atom> {
    prefix.value()
}
#[wasm_bindgen]
/// Returns the element's category
pub fn categories(element: &ElementId) -> ElementCategory {
    element.categories()
}
#[wasm_bindgen]
/// Returns the attribute's group
pub fn groups(attr: &AttrId) -> AttributeGroup {
    attr.attribute_group()
}
#[wasm_bindgen(js_name = permittedCategories)]
/// Returns element categories allowed as children
pub fn permitted_categories(element: &ElementId) -> ElementCategory {
    element.permitted_categories()
}
#[wasm_bindgen(js_name = permittedElements)]
/// Returns specific elements allowed as children
pub fn permitted_elements(element: &ElementId) -> Vec<ElementId> {
    element.permitted_elements().to_vec()
}
#[wasm_bindgen(js_name = expectedAttributes)]
/// Returns specific attributes allowed for this element.
pub fn expected_attributes(element: &ElementId) -> Vec<AttrId> {
    element.expected_attributes().to_vec()
}
#[wasm_bindgen(js_name = expectedAttributeGroups)]
/// Returns attribute groups allowed for this element.
pub fn expected_attribute_groups(element: &ElementId) -> AttributeGroup {
    element.expected_attribute_groups()
}
#[wasm_bindgen(js_name = elementCategoryNames)]
/// Returns the names of each flag set in the categories
pub fn element_category_names(categories: &ElementCategory) -> Vec<String> {
    categories
        .iter_names()
        .map(|(n, _)| n.to_string())
        .collect()
}
#[wasm_bindgen(js_name = attributeGroupNames)]
/// Returns the names of each flag set in the groups
pub fn attribute_group_names(groups: &AttributeGroup) -> Vec<String> {
    groups.iter_names().map(|(n, _)| n.to_string()).collect()
}

#[wasm_bindgen]
#[allow(clippy::struct_field_names)]
/// An actor holds a reference to a document to act upon.
///
/// The actor will embed it's state into the document upon parsing and serializing.
pub struct Actor {
    actor: oxvg_actions::Actor<'static, 'static>,
    #[cfg(feature = "roxmltree")]
    source_ptr: *mut str,
    #[cfg(feature = "roxmltree")]
    xml_ptr: *mut roxmltree::Document<'static>,
    arena_ptr: *mut oxvg_ast::arena::Arena<'static, 'static>,
    values_ptr: *mut oxvg_ast::arena::Values,
}

type DerivedState = oxvg_actions::DerivedState<'static>;
type Error = oxvg_actions::Error<'static>;
type Action = oxvg_actions::Action<'static>;

#[wasm_bindgen]
impl Actor {
    #[cfg(feature = "roxmltree")]
    #[wasm_bindgen(constructor)]
    /// Creates a new actor with a reference to the document. The state of the actor will be
    /// derived from the document's `oxvg:state` element.
    ///
    /// # Errors
    ///
    /// If the document cannot be parsed.
    pub fn new(document: String) -> Result<Self, Error> {
        let source = Box::leak(document.into_boxed_str());
        let source_ptr = std::ptr::from_mut(source);
        let xml = Box::leak(Box::new(
            roxmltree::Document::parse(source).map_err(|err| Error::ParseError(err.to_string()))?,
        ));
        let xml_ptr = std::ptr::from_mut(xml);

        let values = Box::leak(Box::new(Allocator::new_values()));
        let arena = Box::leak(Box::new(Allocator::new_arena_with_capacity(
            xml.descendants().len(),
        )));
        let values_ptr = std::ptr::from_mut(values);
        let arena_ptr = std::ptr::from_mut(arena);

        let (root, arena) =
            parse_tree_with_allocator(xml, arena, values, |root, arena| (root, arena))
                .map_err(|err| Error::ParseError(err.to_string()))?;

        Ok(Self {
            actor: oxvg_actions::Actor::new(root, arena)?,
            source_ptr,
            xml_ptr,
            arena_ptr,
            values_ptr,
        })
    }

    #[cfg(feature = "web_sys")]
    #[wasm_bindgen(constructor)]
    /// Creates a new actor with a reference to the document. The state of the actor will be
    /// derived from the document's `oxvg:state` element.
    ///
    /// # Errors
    ///
    /// If the document cannot be parsed.
    pub fn new(document: &Document) -> Result<Self, Error> {
        let values = Box::leak(Box::new(Allocator::new_values()));
        let arena = Box::leak(Box::new(Allocator::new_arena()));
        let values_ptr = std::ptr::from_mut(values);
        let arena_ptr = std::ptr::from_mut(arena);

        let (root, arena) =
            parse_tree_with_allocator(document, arena, values, |root, arena| (root, arena))
                .map_err(|err| Error::ParseError(err.to_string()))?;

        Ok(Self {
            actor: oxvg_actions::Actor::new(root, arena)?,
            arena_ptr,
            values_ptr,
        })
    }

    /// Returns a rich state object based on the `oxvg:state` embedded in the document
    ///
    /// # Errors
    ///
    /// If the state element is invalid
    #[wasm_bindgen(js_name = deriveState)]
    pub fn derive_state(&self) -> Result<DerivedState, Error> {
        self.actor.derive_state()
    }

    /// Executes the given action and it's arguments upon the document.
    ///
    /// # Errors
    ///
    /// If the given action fails
    #[wasm_bindgen]
    pub fn dispatch(&mut self, action: Action) -> Result<(), Error> {
        self.actor.dispatch(action)
    }

    /// Sets the attribute to selected elements.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    #[wasm_bindgen]
    pub fn attr(&mut self, name: &str, value: &str) -> Result<(), Error> {
        self.actor.attr(name, value)
    }

    /// Toggles the class-name on selected elements.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    #[wasm_bindgen]
    pub fn class(&mut self, name: &str) -> Result<(), Error> {
        self.actor.class(name)
    }

    /// Removes OXVG state from the document
    #[wasm_bindgen]
    pub fn forget(&mut self) {
        self.actor.forget();
    }

    /// Updates the state of the actor to point to the elements matching the given selector.
    /// Elements can also be selected by a space/comma separated list of allocation-id
    /// integers.
    ///
    /// # Errors
    ///
    /// If query is invalid
    #[wasm_bindgen]
    pub fn select(&mut self, query: &str) -> Result<(), Error> {
        self.actor.select(query)
    }

    /// Updates the state of the actor to point to the elements matching the given selector,
    /// including any previous selections.
    /// Elements can also be selected by a space/comma separated list of allocation-id
    /// integers.
    ///
    /// # Errors
    ///
    /// If query is invalid
    #[wasm_bindgen]
    #[wasm_bindgen(js_name = selectMore)]
    pub fn select_more(&mut self, query: &str) -> Result<(), Error> {
        self.actor.select_more(query)
    }

    /// Updates the state of the actor to deselected any selected nodes.
    #[wasm_bindgen]
    pub fn deselect(&mut self) {
        self.actor.deselect();
    }

    /// Returns the actor's updated document as a string
    ///
    /// # Errors
    ///
    /// If serializaton fails.
    pub fn document(&self, minify: Option<bool>) -> Result<String, String> {
        let options = if matches!(minify, Some(true)) {
            Options::default()
        } else {
            Options::original()
        };
        self.actor
            .root
            .serialize_with_options(options)
            .map_err(|err| err.to_string())
    }
}

impl Drop for Actor {
    fn drop(&mut self) {
        unsafe {
            #[cfg(feature = "roxmltree")]
            drop(Box::from_raw(self.source_ptr));
            #[cfg(feature = "roxmltree")]
            drop(Box::from_raw(self.xml_ptr));
            drop(Box::from_raw(self.arena_ptr));
            drop(Box::from_raw(self.values_ptr));
        }
    }
}
