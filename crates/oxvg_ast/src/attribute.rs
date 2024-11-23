use std::fmt::Debug;

use crate::{atom::Atom, name::Name};

pub trait Attr:
    PartialEq + From<(Self::Name, Self::Atom)> + From<(<Self::Name as Name>::LocalName, Self::Atom)>
{
    type Name: Name;
    type Atom: Atom;

    fn local_name(&self) -> <Self::Name as Name>::LocalName {
        self.name().local_name()
    }

    fn name(&self) -> Self::Name;

    fn prefix(&self) -> Option<<Self::Name as Name>::Prefix> {
        self.name().prefix()
    }

    fn value(&self) -> Self::Atom;

    fn value_ref(&self) -> &str;

    fn set_value(&mut self, value: Self::Atom) -> Self::Atom;
}

/// <https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap>
pub trait Attributes<'a>: Debug + Clone {
    type Attribute: Attr;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Attribute>;

    fn get_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute>;

    fn get_named_item_ns(
        &self,
        namespace: &<<Self::Attribute as Attr>::Name as Name>::Namespace,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute>;

    fn item(&self, index: usize) -> Option<Self::Attribute>;

    fn remove_named_item(&self, name: &<Self::Attribute as Attr>::Name) -> Option<Self::Attribute>;

    fn remove_named_item_local(
        &self,
        name: &<<Self::Attribute as Attr>::Name as Name>::LocalName,
    ) -> Option<Self::Attribute>;

    fn set_named_item(&self, attr: Self::Attribute) -> Option<Self::Attribute>;

    fn set_named_item_qual(
        &self,
        name: <Self::Attribute as Attr>::Name,
        value: <Self::Attribute as Attr>::Atom,
    ) -> Option<Self::Attribute>;

    fn iter(&self) -> impl Iterator<Item = Self::Attribute>;
}
