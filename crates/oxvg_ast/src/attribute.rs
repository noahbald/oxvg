//! XML element attribute traits.
use std::cell::{self, Ref, RefCell, RefMut};

use data::{Attr, AttrId};

use crate::{atom::Atom, is_prefix, name::NS};

pub use group::{AttributeGroup, AttributeInfo};

pub mod content_type;
pub mod data;
mod group;

#[macro_export]
/// Returns whether the attribute matches the given names
///
/// # Examples
///
/// ```
/// use oxvg_ast::{
///   atom::Atom,
///   attribute::data::Attr,
///   is_attribute,
/// };
/// use lightningcss::properties::svg::SVGPaint;
///
/// // Matching attribute ids
/// let attr = Attr::Id(Atom::Static("my-attr"));
/// assert!(is_attribute!(attr, Id | Class));
/// ```
///
/// ```
/// use oxvg_ast::{
///   attribute::data::{Attr, inheritable::Inheritable},
///   is_attribute,
/// };
/// use lightningcss::properties::svg::SVGPaint;
///
/// // Matching attribute values
/// let attr = Attr::Stroke(Inheritable::Defined(SVGPaint::None));
/// assert!(is_attribute!(attr, Stroke(Inheritable::Defined(SVGPaint::None))));
/// ```
macro_rules! is_attribute {
    ($attr:expr, $($name:ident $(|)?)+$(,)?) => {
        matches!($attr.name().unaliased(), $(| $crate::attribute::data::AttrId::$name)+)
    };
    ($attr:expr, $name:ident($value:pat)$(,)?) => {
        matches!($attr.unaliased(), $crate::attribute::data::Attr::$name($value))
    };
}

#[macro_export]
/// Returns whether the given attribute is on the [`crate::element::Element`] or [`Attributes`]
macro_rules! has_attribute {
    ($element:expr, $attr:ident$(,)?) => {
        $element.has_attribute(&$crate::attribute::data::AttrId::$attr)
    };
    ($element:expr, $($attr:ident $(|)?)+$(,)?) => {
        $element.attributes().into_iter().any(
            |attr|
            matches!(attr.name().unaliased(), $(| $crate::attribute::data::AttrId::$attr)+)
        )
    }
}

#[macro_export]
/// Sets the given attribute to the [`crate::element::Element`] or [`Attributes`]
macro_rules! set_attribute {
    ($element:expr, $attr:ident$(($inner:expr))?$(,)?) => {
        $element.set_attribute($crate::attribute::data::Attr::$attr$(($inner))?)
    };
}

#[macro_export]
/// Gets the given attribute from the [`crate::element::Element`] or [`Attributes`]
macro_rules! get_attribute {
    ($element:expr, $attr:ident$(,)?) => {
        $element
            .get_attribute(&$crate::attribute::data::AttrId::$attr)
            .and_then(|attr| {
                std::cell::Ref::filter_map(attr, |attr| match attr.unaliased() {
                    $crate::attribute::data::Attr::$attr(inner) => Some(inner),
                    $crate::attribute::data::Attr::Unparsed { .. } => None,
                    _ => unreachable!("{attr:?} did not match {}", stringify!($attr)),
                }).ok()
            })
    };
    ($element:expr, $($attr:ident $(|)?)+$(,)?) => {{
        let attributes = $element.attributes();
        (
            $(get_attribute!($element, $attr),)+
        )
    }};
}

#[macro_export]
/// Mutably gets the given attribute from the element
macro_rules! get_attribute_mut {
    ($element:expr, $attr:ident$(,)?) => {
        $element
            .get_attribute_node_mut(&$crate::attribute::data::AttrId::$attr)
            .and_then(|attr| {
                std::cell::RefMut::filter_map(attr, |attr| match attr {
                    $crate::attribute::data::Attr::$attr(inner) => Some(inner),
                    $crate::attribute::data::Attr::Unparsed { .. } => None,
                    _ => unreachable!(),
                })
                .ok()
            })
    };
}

#[macro_export]
/// Removes the given attribute from the element
macro_rules! remove_attribute {
    ($element:expr, $attr:ident$(,)?) => {
        $element
            .remove_attribute(&$crate::attribute::data::AttrId::$attr)
            .and_then(|attr| match attr {
                $crate::attribute::data::Attr::$attr(inner) => Some(inner),
                $crate::attribute::data::Attr::Unparsed { .. } => None,
                $crate::attribute::data::Attr::Aliased { value, .. } => match *value {
                    $crate::attribute::data::Attr::$attr(inner) => Some(inner),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            })
    };
}

#[derive(Clone)]
/// A representation of a collection of [Attr] objects.
///
/// [MDN | NamedNodeMap](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap)
pub struct Attributes<'a, 'input>(pub &'a RefCell<Vec<Attr<'input>>>);

impl<'a, 'input> Attributes<'a, 'input> {
    /// The number of attributes stored in the collection.
    ///
    /// [MDN | length](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/length)
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    /// Whether there are any attributes stored in the collection
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an attribute corresponding to the given name.
    ///
    /// [MDN | getNamedItem](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/getNamedItem)
    pub fn get_named_item(&self, name: &AttrId) -> Option<cell::Ref<'a, Attr<'input>>> {
        cell::Ref::filter_map(self.0.borrow(), |v: &Vec<Attr<'input>>| {
            v.iter().find(|a| a.name() == name)
        })
        .ok()
    }

    /// See [`Attributes::get_named_item`]
    pub fn get_named_item_local(&self, local_name: &Atom) -> Option<cell::Ref<'a, Attr<'input>>> {
        cell::Ref::filter_map(self.0.borrow(), |v: &Vec<Attr<'input>>| {
            v.iter()
                .find(|a| a.prefix().is_empty() && a.local_name() == local_name)
        })
        .ok()
    }

    /// See [`Attributes::get_named_item`]
    pub fn get_named_item_mut(&self, name: &AttrId) -> Option<RefMut<'a, Attr<'input>>> {
        RefMut::filter_map(self.0.borrow_mut(), |v: &mut Vec<Attr<'input>>| {
            v.iter_mut()
                .find(|a| a.prefix() == name.prefix() && a.local_name() == name.local_name())
        })
        .ok()
    }

    /// See [`Attributes::get_named_item_ns`]
    pub fn get_named_item_ns(
        &self,
        namespace: &NS,
        local_name: &Atom,
    ) -> Option<cell::Ref<'a, Attr<'input>>> {
        cell::Ref::filter_map(self.0.borrow(), |v: &Vec<Attr<'input>>| {
            v.iter()
                .find(|a| a.prefix().is_ns(namespace) && a.local_name() == local_name)
        })
        .ok()
    }

    /// Returns the attribute in the collection matching the index
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/item)
    pub fn item(&self, index: usize) -> Option<cell::Ref<'a, Attr<'input>>> {
        cell::Ref::filter_map(self.0.borrow(), |v: &Vec<Attr<'input>>| v.get(index)).ok()
    }

    /// Returns the mutable attribute in the collection matching the index
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/item)
    fn item_mut(&self, index: usize) -> Option<RefMut<'a, Attr<'input>>> {
        RefMut::filter_map(self.0.borrow_mut(), |v: &mut Vec<Attr<'input>>| {
            v.get_mut(index)
        })
        .ok()
    }

    /// Removes the attribute corresponding to the given name from the collection.
    ///
    /// [MDN | removeNamedItem](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/removeNamedItem)
    pub fn remove_named_item(&self, name: &AttrId) -> Option<Attr<'input>> {
        let mut attrs = self.0.borrow_mut();
        let index = attrs.iter().position(|a| a.name() == name)?;
        Some(attrs.remove(index))
    }

    /// Puts the attribute identified by it's name in the collection. If there's already an attribute with
    /// the same name, it is replaced.
    ///
    /// [MDN | setNamedItem](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/setNamedItem)
    pub fn set_named_item(&self, attr: Attr<'input>) -> Option<Attr<'input>> {
        let attrs = &mut *self.0.borrow_mut();
        if let Some(index) = attrs
            .iter()
            .position(|a| a.prefix() == attr.prefix() && a.local_name() == attr.local_name())
        {
            Some(std::mem::replace(&mut attrs[index], attr))
        } else {
            attrs.push(attr);
            None
        }
    }

    // For use in macros interoperable with `Element`
    #[doc(hidden)]
    pub fn set_attribute(&self, attr: Attr<'input>) -> Option<Attr<'input>> {
        self.set_named_item(attr)
    }

    // For use in macros interoperable with `Element`
    #[doc(hidden)]
    pub fn get_attribute(&self, name: &AttrId) -> Option<cell::Ref<'a, Attr<'input>>> {
        self.get_named_item(name)
    }

    // For use in macros interoperable with `Element`
    #[doc(hidden)]
    pub fn has_attribute(&self, name: &AttrId) -> bool {
        self.get_named_item(name).is_some()
    }

    // For use in macros interoperable with `Element`
    #[doc(hidden)]
    pub fn attributes(&self) -> &Self {
        self
    }

    // For use in macros interoperable with `Element`
    #[doc(hidden)]
    pub fn get_attribute_node_mut(&self, name: &AttrId) -> Option<RefMut<'a, Attr<'input>>> {
        self.get_named_item_mut(name)
    }

    // For use in macros interoperable with `Element`
    #[doc(hidden)]
    pub fn remove_attribute(&self, name: &AttrId) -> Option<Attr<'input>> {
        self.remove_named_item(name)
    }

    /// Returns an new iterator that mutably goes over each attribute in the collection.
    pub fn into_iter_mut(self) -> AttributesIterMut<'a, 'input> {
        AttributesIterMut::new(self)
    }

    /// Sorts attributes with the following behaviour.
    ///
    /// 1. Keeps the `xmlns` attribute at the front if `xmlns_front` is `true`
    /// 2. Orders `xmlns` prefixed attributes at the front
    /// 3. Orders prefixed attributes after `xmlns` prefixed attributes
    /// 4. Orders attributes matching `order` after based on the order of the list
    /// 5. Sorts attributes in each order alphabetically
    pub fn sort(&self, order: &[impl std::ops::Deref<Target = str>], xmlns_front: bool) {
        fn get_ns_priority(attr: &AttrId, xmlns_front: bool) -> usize {
            if xmlns_front {
                if is_attribute!(attr, XMLNS) {
                    return 3;
                }
                if is_prefix!(attr, XMLNS) {
                    return 2;
                }
            }
            if !is_prefix!(attr, SVG) {
                return 1;
            }
            0
        }

        self.0.borrow_mut().sort_by(|a, b| {
            let a_name = a.name();
            let b_name = b.name();
            let a_priority = get_ns_priority(a_name, xmlns_front);
            let b_priority = get_ns_priority(b_name, xmlns_front);
            if a_priority != b_priority {
                return b_priority.cmp(&a_priority);
            }

            let a_part = a
                .local_name()
                .split_once('-')
                .map_or_else(|| a.local_name().as_ref(), |p| p.0);
            let b_part = b
                .local_name()
                .split_once('-')
                .map_or_else(|| b.local_name().as_ref(), |p| p.0);
            if a_part != b_part {
                let a_in_order = order.iter().position(|x| &**x == a_part);
                let b_in_order = order.iter().position(|x| &**x == b_part);
                if a_in_order.is_some() && b_in_order.is_some() {
                    return a_in_order.cmp(&b_in_order);
                }
                if a_in_order.is_some() {
                    return std::cmp::Ordering::Less;
                }
                if b_in_order.is_some() {
                    return std::cmp::Ordering::Greater;
                }
            }

            a_name.cmp(b_name)
        });
    }

    /// Iterates through the attributes and only keeps those where the callback is
    /// evaluated to be `true`.
    pub fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&Attr<'input>) -> bool,
    {
        self.0.borrow_mut().retain(|attr| f(attr));
    }
}

impl<'a, 'input> IntoIterator for Attributes<'a, 'input> {
    type Item = Ref<'a, Attr<'input>>;
    type IntoIter = AttributesIter<'a, 'input>;

    fn into_iter(self) -> Self::IntoIter {
        AttributesIter::new(self)
    }
}

impl std::fmt::Debug for Attributes<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_tuple = f.debug_tuple("Attributes");
        for a in self.0.borrow().iter() {
            debug_tuple.field(a);
        }
        debug_tuple.finish()
    }
}

macro_rules! define_attrs_iter {
    ($name:ident$((Ref<'a, $deref:ident>))?$((RefMut<'a, $derefmut:ident>))?) => {
        /// An iterator goes through each attribute of an [Attributes] struct.
        pub struct $name<'a, 'input> {
            index: usize,
            attributes: Attributes<'a, 'input>,
        }

        impl<'a, 'input> $name<'a, 'input> {
            /// Creates an iterator that goes from start to the of the given attributes.
            pub fn new(attributes: Attributes<'a, 'input>) -> Self {
                Self {
                    index: 0,
                    attributes,
                }
            }
        }

        impl<'a, 'input> Iterator for $name<'a, 'input> {
            $(type Item = Ref<'a, $deref<'input>>;)?
            $(type Item = RefMut<'a, $derefmut<'input>>;)?

            fn next(&mut self) -> Option<Self::Item> {
                $(let output: Option<Ref<'a, $deref<'input>>> = self.attributes.item(self.index);)?
                $(let output: Option<RefMut<'a, $derefmut<'input>>> = self.attributes.item_mut(self.index);)?
                self.index += 1;
                output
            }
        }
    };
}

define_attrs_iter!(AttributesIter(Ref<'a, Attr>));
define_attrs_iter!(AttributesIterMut(RefMut<'a, Attr>));
