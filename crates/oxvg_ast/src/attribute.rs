//! XML element attribute traits.
use std::{
    cell::{self, Ref, RefCell, RefMut},
    fmt::Display,
};

use data::{Attr, AttrId};

use crate::{
    atom::Atom,
    name::{Prefix, NS},
};

pub use group::AttributeGroup;

pub mod data;
mod group;

/// Writes the attribute as a name and quoted value (unescaped) separated by `'='`
pub struct Formatter<'a, 'input>(&'a Attr<'input>);

impl Display for Formatter<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(r#"{}="{}""#, self.0.name(), self.0.value()))
    }
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
    fn len(&self) -> usize {
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

    pub fn get_named_item_ns(
        &self,
        namespace: &NS<'input>,
        local_name: &Atom<'input>,
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
    fn item(&self, index: usize) -> Option<cell::Ref<'a, Attr<'input>>> {
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

    /// Returns an new iterator that goes over each attribute in the collection.
    pub fn into_iter(self) -> AttributesIter<'a, 'input> {
        AttributesIter::new(self)
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
    fn sort(&self, order: &[String], xmlns_front: bool) {
        fn get_ns_priority(attr: &Attr, xmlns_front: bool) -> usize {
            if xmlns_front {
                if *attr.name() == AttrId::XMLNS {
                    return 3;
                }
                if *attr.prefix() == Prefix::XMLNS {
                    return 2;
                }
            }
            if *attr.prefix() != Prefix::SVG {
                return 1;
            }
            0
        }

        self.0.borrow_mut().sort_by(|a, b| {
            let a_priority = get_ns_priority(a, xmlns_front);
            let b_priority = get_ns_priority(b, xmlns_front);
            let priority_ord = b_priority.cmp(&a_priority);
            if priority_ord != std::cmp::Ordering::Equal {
                return priority_ord;
            }

            let a_prefix = a.prefix();
            let b_prefix = b.prefix();
            let a = a.local_name();
            let a_part = a.split_once('-').map_or_else(|| a.as_ref(), |p| p.0);
            let b = b.local_name();
            let b_part = b.split_once('-').map_or_else(|| b.as_ref(), |p| p.0);
            if a_part != b_part {
                let a_in_order = order.iter().position(|x| x == a_part);
                let b_in_order = order.iter().position(|x| x == b_part);
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

            a_prefix.cmp(&b_prefix).cmp(&a.cmp(&b))
        });
    }

    /// Iterates through the attributes and only keeps those where the callback is
    /// evaluated to be `true`.
    fn retain<F>(&self, mut f: F)
    where
        F: FnMut(&Attr<'input>) -> bool,
    {
        self.0.borrow_mut().retain(|attr| f(attr));
    }
}

impl std::fmt::Debug for Attributes<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Attributes5Ever { ")?;
        self.0
            .borrow()
            .iter()
            .try_for_each(|a| f.write_fmt(format_args!(r#"{}="{}" "#, a.name(), a.value())))?;
        f.write_str("} ")
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
