//! XML element attribute traits.
use std::{
    cell::{Ref, RefMut},
    fmt::{Debug, Display},
    marker::PhantomData,
};

use crate::{atom::Atom, name::Name};

/// Represents one of an element's attributes.
///
/// [MDN | Attr](https://developer.mozilla.org/en-US/docs/Web/API/Attr)
pub trait Attr: PartialEq + Debug + Sized + Clone {
    /// The type representing the name of an attribute (e.g. `foo` of `foo="bar"`)
    type Name: Name;
    /// The type representing the value of an attribute (e.g. `"bar"` of `foo="bar"`)
    type Atom: Atom;

    /// Creates a new attribute
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

    /// Mutably returns the qualified name of an attribute.
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

    /// Mutably returns the value of the attribute
    fn value_mut(&mut self) -> &mut Self::Atom;

    /// Overwrites the value of the attribute with a new one.
    fn set_value(&mut self, value: Self::Atom) -> Self::Atom;

    /// Pushes to the end of the value.
    fn push(&mut self, value: &Self::Atom);

    /// Gets a substring of the attribute's value.
    fn sub_value(&self, offset: u32, length: u32) -> Self::Atom;

    #[cfg(feature = "style")]
    /// Returns the attribute as a presentation attribute with a CSS value,
    /// similar to [`lightningcss::properties::Property`]
    fn presentation(&self) -> Option<crate::style::PresentationAttr> {
        if self.prefix().is_some() {
            return None;
        }
        let id = crate::style::PresentationAttrId::from(self.local_name().as_ref());
        crate::style::PresentationAttr::parse_string(
            id,
            self.value(),
            lightningcss::stylesheet::ParserOptions::default(),
        )
        .ok()
    }

    /// Returns an object that can write an attribute as [Display]
    fn formatter(&self) -> Formatter<'_, Self> {
        Formatter(self)
    }
}

/// Writes the attribute as a name and quoted value (unescaped) separated by `'='`
pub struct Formatter<'a, A: Attr>(&'a A);

impl<'a, A: Attr> Display for Formatter<'a, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            r#"{}="{}""#,
            self.0.name().formatter(),
            self.0.value()
        ))
    }
}

/// A representation of a collection of [Attr] objects.
///
/// [MDN | NamedNodeMap](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap)
pub trait Attributes<'a>: Debug + Clone {
    /// The type of an attribute contained by the collection of attributes.
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
    fn get_named_item(
        &self,
        name: &<Self::Attribute as Attr>::Name,
    ) -> Option<Ref<'a, Self::Attribute>>;

    /// See [`Attributes::get_named_item`]
    fn get_named_item_mut(
        &self,
        name: &<Self::Attribute as Attr>::Name,
    ) -> Option<RefMut<'a, Self::Attribute>>;

    /// Returns an attribute corresponding to the given local-name, only if the attribute has no
    /// prefix
    fn get_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, Self::Attribute>>;

    /// See [`Attributes::get_named_item_local`]
    fn get_named_item_local_mut(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<RefMut<'a, Self::Attribute>>;

    /// Returns the attribute corresponding to the given local-name in the given namespace
    ///
    /// [MDN | getNamedItemNS](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/getNamedItemNS)
    fn get_named_item_ns(
        &self,
        namespace: &<<Self::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Ref<'a, Self::Attribute>>;

    /// Returns the attribute in the collection matching the index
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/item)
    fn item(&self, index: usize) -> Option<Ref<'a, Self::Attribute>>;

    /// Returns the mutable attribute in the collection matching the index
    ///
    /// [MDN | item](https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap/item)
    fn item_mut(&self, index: usize) -> Option<RefMut<'a, Self::Attribute>>;

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
    ) -> Option<Self::Attribute> {
        let attr = Self::Attribute::new(name, value);
        self.set_named_item(attr)
    }

    /// Returns an new iterator that goes over each attribute in the collection.
    fn into_iter(self) -> AttributesIter<'a, Self> {
        AttributesIter::new(self)
    }

    /// Returns an new iterator that mutably goes over each attribute in the collection.
    fn into_iter_mut(self) -> AttributesIterMut<'a, Self> {
        AttributesIterMut::new(self)
    }

    /// Sorts attributes with the following behaviour.
    ///
    /// 1. Keeps the `xmlns` attribute at the front if `xmlns_front` is `true`
    /// 2. Orders `xmlns` prefixed attributes at the front
    /// 3. Orders prefixed attributes after `xmlns` prefixed attributes
    /// 4. Orders attributes matching `order` after based on the order of the list
    /// 5. Sorts attributes in each order alphabetically
    fn sort(&self, order: &[String], xmlns_front: bool);

    /// Iterates through the attributes and only keeps those where the callback is
    /// evaluated to be `true`.
    fn retain<F>(&self, f: F)
    where
        F: FnMut(&Self::Attribute) -> bool;
}

macro_rules! define_attrs_iter {
    ($name:ident$((Ref<'a, $deref:ident>))?$((RefMut<'a, $derefmut:ident>))?) => {
        /// An iterator goes through each attribute of an [Attributes] struct.
        pub struct $name<'a, A: Attributes<'a>>
        where
            A: Attributes<'a>,
            A::Attribute: 'a,
        {
            index: usize,
            attributes: A,
            ph: PhantomData<&'a ()>,
        }

        impl<'a, A: Attributes<'a>> $name<'a, A> {
            /// Creates an iterator that goes from start to the of the given attributes.
            pub fn new(attributes: A) -> Self {
                Self {
                    index: 0,
                    attributes,
                    ph: PhantomData,
                }
            }
        }

        impl<'a, A: Attributes<'a>> Iterator for $name<'a, A> {
            $(type Item = Ref<'a, <$deref as Attributes<'a>>::Attribute>;)?
            $(type Item = RefMut<'a, <$derefmut as Attributes<'a>>::Attribute>;)?

            fn next(&mut self) -> Option<Self::Item> {
                $(let output: Option<Ref<'a, <$deref as Attributes<'a>>::Attribute>> = self.attributes.item(self.index);)?
                $(let output: Option<RefMut<'a, <$derefmut as Attributes<'a>>::Attribute>> = self.attributes.item_mut(self.index);)?
                self.index += 1;
                output
            }
        }
    };
}

define_attrs_iter!(AttributesIter(Ref<'a, A>));
define_attrs_iter!(AttributesIterMut(RefMut<'a, A>));
