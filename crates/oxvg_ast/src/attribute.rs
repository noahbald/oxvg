use std::fmt::{Debug, Display};

use crate::{atom::Atom, name::Name};

/// Represents one of an element's attributes.
///
/// [MDN | Attr](https://developer.mozilla.org/en-US/docs/Web/API/Attr)
pub trait Attr:
    PartialEq
    + From<(Self::Name, Self::Atom)>
    + From<(<Self::Name as Name>::LocalName, Self::Atom)>
    + Display
    + Debug
{
    type Name: Name;
    type Atom: Atom;

    /// Returns the local part of the qualified name of an attribute.
    ///
    /// [MDN | localName](https://developer.mozilla.org/en-US/docs/Web/API/Attr/localName)
    fn local_name(&self) -> <Self::Name as Name>::LocalName {
        self.name().local_name()
    }

    /// Returns the qualified name of an attribute.
    ///
    /// [MDN | name](https://developer.mozilla.org/en-US/docs/Web/API/Attr/name)
    fn name(&self) -> Self::Name;

    /// Returns the namespace prefix of the attribute.
    ///
    /// [MDN | prefix](https://developer.mozilla.org/en-US/docs/Web/API/Attr/prefix)
    fn prefix(&self) -> Option<<Self::Name as Name>::Prefix> {
        self.name().prefix()
    }

    /// Returns the value of the attribute.
    ///
    /// [MDN | value](https://developer.mozilla.org/en-US/docs/Web/API/Attr/value)
    fn value(&self) -> Self::Atom;

    /// Returns the value of the attribute as a string slice.
    fn value_ref(&self) -> &str;

    /// Overwrites the value of the attribute with a new one.
    fn set_value(&mut self, value: Self::Atom) -> Self::Atom;

    /// Converts a reference to an attribute to an owned one, usually by cloning
    fn into_owned(self) -> Self;

    fn presentation(&self) -> Option<crate::style::PresentationAttr>;
}

/// A representation of a collection of [Attr] objects.
///
/// [MDN | NamedNodeMap](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap)
pub trait Attributes<'a>: Debug + Clone {
    type Attribute: Attr;

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
    fn get_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Attribute>;

    /// Returns an attribute corresponding to the given local-name.
    /// Note that unlike the browser, this will match an attribute regardless of the prefix. For
    /// more browser accurate behaviour, try using [`Attributes::get_named_item`].
    fn get_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute>;

    /// Returns the attribute corresponding to the given local-name in the given namespace
    ///
    /// [MDN | getNamedItemNS](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/getNamedItemNS)
    fn get_named_item_ns(
        &self,
        namespace: &<<Self::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute>;

    /// Returns the attribute in the collection matching the index
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/item)
    fn item(&self, index: usize) -> Option<Self::Attribute>;

    /// Removes the attribute corresponding to the given name from the collection.
    ///
    /// [MDN | removeNamedItem](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/removeNamedItem)
    fn remove_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Attribute>;

    /// Removes the attribute corresponding to the given local-name from the collection.
    ///
    /// Similarly to [`Attributes::get_named_item_local`], this will ignore the prefix of an
    /// attribute.
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
    fn iter(&self) -> impl Iterator<Item = Self::Attribute>;

    fn sort(&self, order: &[String], xmlns_front: bool);

    fn retain<F>(&self, f: F)
    where
        F: FnMut(Self::Attribute) -> bool;
}
