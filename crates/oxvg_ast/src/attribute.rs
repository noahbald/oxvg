use std::fmt::Debug;

use crate::{atom::Atom, name::Name};

pub trait Attr: PartialEq {
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
}

/// <https://developer.mozilla.org/en-US/docs/Web/API/NamedNodeMap>
pub trait Attributes<'a>: Debug {
    type Attribute: Attr;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_named_item(
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

    fn set_named_item(&self, attr: Self::Attribute) -> Option<Self::Attribute>;

    fn iter(&self) -> impl Iterator<Item = Self::Attribute>;
}
