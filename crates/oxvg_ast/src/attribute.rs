use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{atom::Atom, name::Name};

/// Represents one of an element's attributes.
///
/// [MDN | Attr](https://developer.mozilla.org/en-US/docs/Web/API/Attr)
pub trait Attr: PartialEq + Debug + Sized + Clone {
    type Name: Name;
    type Atom: Atom;

    fn new(name: Self::Name, value: Self::Atom) -> Self;

    /// Returns the local part of the qualified name of an attribute.
    ///
    /// [MDN | localName](https://developer.mozilla.org/en-US/docs/Web/API/Attr/localName)
    fn local_name(&self) -> &<Self::Name as Name>::LocalName {
        self.name().local_name()
    }

    /// Returns the qualified name of an attribute.
    ///
    /// [MDN | name](https://developer.mozilla.org/en-US/docs/Web/API/Attr/name)
    fn name(&self) -> &Self::Name;

    fn name_mut(&mut self) -> &mut Self::Name;

    /// Returns the namespace prefix of the attribute.
    ///
    /// [MDN | prefix](https://developer.mozilla.org/en-US/docs/Web/API/Attr/prefix)
    fn prefix(&self) -> &Option<<Self::Name as Name>::Prefix> {
        self.name().prefix()
    }

    /// Returns the value of the attribute.
    ///
    /// [MDN | value](https://developer.mozilla.org/en-US/docs/Web/API/Attr/value)
    fn value(&self) -> &Self::Atom;

    fn value_mut(&mut self) -> &mut Self::Atom;

    /// Overwrites the value of the attribute with a new one.
    fn set_value(&mut self, value: Self::Atom) -> Self::Atom;

    fn presentation(&self) -> Option<crate::style::PresentationAttr>;

    fn formatter(&self) -> Formatter<'_, Self> {
        Formatter(self)
    }
}

pub struct Formatter<'a, A: Attr>(&'a A);

impl<'a, A: Attr> Display for Formatter<'a, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}={}",
            self.0.name().formatter(),
            self.0.value()
        ))
    }
}

/// A representation of a collection of [Attr] objects.
///
/// [MDN | NamedNodeMap](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap)
pub trait Attributes<'a>: Debug + Clone {
    type Attribute: Attr;
    type Deref: Deref<Target = Self::Attribute>;
    type DerefMut: DerefMut<Target = Self::Attribute>;

    /// The number of attributes stored in the collection.
    ///
    /// [MDN | length](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/length)
    fn len(&self) -> usize;

    /// Whether there are any attributes stored in the collection
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an attribute corresponding to the given name.
    ///
    /// [MDN | getNamedItem](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/getNamedItem)
    fn get_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Deref>;

    /// See [`Attributes::get_named_item`]
    fn get_named_item_mut(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::DerefMut>;

    /// Returns an attribute corresponding to the given local-name, only if the attribute has no
    /// prefix
    fn get_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Deref>;

    /// See [`Attributes::get_named_item_local`]
    fn get_named_item_local_mut(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::DerefMut>;

    /// Returns the attribute corresponding to the given local-name in the given namespace
    ///
    /// [MDN | getNamedItemNS](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/getNamedItemNS)
    fn get_named_item_ns(
        &self,
        namespace: &<<Self::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Deref>;

    /// Returns the attribute in the collection matching the index
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/item)
    fn item(&self, index: usize) -> Option<Self::Deref>;

    fn item_mut(&self, index: usize) -> Option<Self::DerefMut>;

    /// Removes the attribute corresponding to the given name from the collection.
    ///
    /// [MDN | removeNamedItem](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/removeNamedItem)
    fn remove_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Attribute>;

    /// Removes the attribute corresponding to the given local-name from the collection, only if
    /// the attribute has no prefix.
    fn remove_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute>;

    /// Puts the attribute identified by it's name in the collection. If there's already an attribute with
    /// the same name, it is replaced.
    ///
    /// [MDN | setNamedItem](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/setNamedItem)
    fn set_named_item(&self, attr: Self::Attribute) -> Option<Self::Attribute>;

    /// Creates an attribute from the given name and value and puts it in the collection similar to
    /// [`Attributes::set_named_item`]
    fn set_named_item_qual(
        &self,
        name: <Self::Attribute as Attr>::Name,
        value: <Self::Attribute as Attr>::Atom,
    ) -> Option<Self::Attribute>;

    /// Returns an new iterator that goes over each attribute in the collection.
    fn into_iter(self) -> AttributesIter<'a, Self>;

    /// Returns an new iterator that mutably goes over each attribute in the collection.
    fn into_iter_mut(self) -> AttributesIterMut<'a, Self>;

    fn sort(&self, order: &[String], xmlns_front: bool);

    fn retain<F>(&self, f: F)
    where
        F: FnMut(&Self::Attribute) -> bool;
}

macro_rules! define_attrs_iter {
    ($name:ident$((ref $deref:ident))?$((mut $derefmut:ident))?) => {
        pub struct $name<'a, A: Attributes<'a>> {
            index: usize,
            attributes: A,
            ph: PhantomData<&'a ()>,
        }

        impl<'a, A: Attributes<'a>> $name<'a, A> {
            pub fn new(attributes: A) -> Self {
                Self {
                    index: 0,
                    attributes,
                    ph: PhantomData,
                }
            }
        }

        impl<'a, A: Attributes<'a>> Iterator for $name<'a, A> {
            $(type Item = $deref::Deref;)?
            $(type Item = $derefmut::DerefMut;)?

            fn next(&mut self) -> Option<Self::Item> {
                $(let output: Option<$deref::Deref> = self.attributes.item(self.index);)?
                $(let output: Option<$derefmut::DerefMut> = self.attributes.item_mut(self.index);)?
                self.index += 1;
                output
            }
        }
    };
}

define_attrs_iter!(AttributesIter(ref A));
define_attrs_iter!(AttributesIterMut(mut A));
