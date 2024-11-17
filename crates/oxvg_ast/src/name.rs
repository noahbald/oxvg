use crate::atom::Atom;

pub trait Name: PartialEq {
    type LocalName: Atom;
    type Prefix: Atom;

    fn local_name(&self) -> Self::LocalName;

    fn prefix(&self) -> Option<Self::Prefix>;
}
