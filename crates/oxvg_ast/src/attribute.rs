use crate::{atom::Atom, name::Name};

pub trait Attr<'b>: PartialEq {
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
pub trait Attributes<'a> {
    type Attribute<'b>: Attr<'b>
    where
        'a: 'b;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_named_item<'b>(
        &self,
        name: <Self::Attribute<'b> as Attr<'b>>::Name,
    ) -> Option<Self::Attribute<'a>>;

    fn item(&self, index: usize) -> Option<Self::Attribute<'a>>;

    fn remove_named_item(
        &self,
        name: &<Self::Attribute<'a> as Attr<'a>>::Name,
    ) -> Option<Self::Attribute<'_>>;

    fn set_named_item(&self, attr: Self::Attribute<'a>) -> Option<Self::Attribute<'_>>;

    fn iter(&'a self) -> impl Iterator<Item = Self::Attribute<'a>> + 'a;
}
