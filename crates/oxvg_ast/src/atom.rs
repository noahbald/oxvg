use std::ops::Deref;

use lightningcss::values::string::CowArcStr;

use crate::{parse::Parse, serialize::ToAtom};

#[derive(Debug, Clone, Hash)]
pub enum Atom<'input> {
    Static(&'static str),
    Cow(CowArcStr<'input>),
    #[cfg(feature = "markup5ever")]
    NS(string_cache::Atom<xml5ever::NamespaceStaticSet>),
    #[cfg(feature = "markup5ever")]
    Prefix(string_cache::Atom<xml5ever::PrefixStaticSet>),
    #[cfg(feature = "markup5ever")]
    Local(string_cache::Atom<xml5ever::LocalNameStaticSet>),
    #[cfg(feature = "markup5ever")]
    Tendril(tendril::StrTendril),
}

impl Atom<'_> {
    fn into_owned<'any>(self) -> Atom<'any>
    where
        'static: 'any,
    {
        match self {
            Self::Static(str) => str.into(),
            Self::Cow(cow) => cow.to_string().into(),
            Self::NS(ns) => ns.into(),
            Self::Prefix(prefix) => prefix.into(),
            Self::Local(local) => local.into(),
            Self::Tendril(tendril) => tendril.into(),
        }
    }
}

impl Default for Atom<'_> {
    fn default() -> Self {
        Self::Cow("".into())
    }
}

impl Deref for Atom<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Static(str) => str,
            Self::Cow(str) => str,
            #[cfg(feature = "markup5ever")]
            Self::NS(str) => str,
            #[cfg(feature = "markup5ever")]
            Self::Prefix(str) => str,
            #[cfg(feature = "markup5ever")]
            Self::Local(str) => str,
            #[cfg(feature = "markup5ever")]
            Self::Tendril(str) => str,
        }
    }
}

impl Eq for Atom<'_> {}
impl PartialEq for Atom<'_> {
    fn eq(&self, other: &Self) -> bool {
        let deref: &str = &*self;
        deref.eq(&**other)
    }
}

impl Ord for Atom<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let deref: &str = &*self;
        deref.cmp(&**other)
    }
}
impl PartialOrd for Atom<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let deref: &str = &*self;
        deref.partial_cmp(&**other)
    }
}

impl<'input> From<&'input str> for Atom<'input> {
    fn from(value: &'input str) -> Self {
        Self::Cow(value.into())
    }
}

impl From<String> for Atom<'_> {
    fn from(value: String) -> Self {
        Self::Cow(value.into())
    }
}

impl<'input> From<CowArcStr<'input>> for Atom<'input> {
    fn from(value: CowArcStr<'input>) -> Self {
        Self::Cow(value)
    }
}

impl<'input> Parse<'input> for Atom<'input> {
    fn parse<'t>(
        input: &mut cssparser_lightningcss::Parser<'input, 't>,
    ) -> Result<
        Self,
        cssparser_lightningcss::ParseError<'input, crate::error::ParseErrorKind<'input>>,
    > {
        Ok(Self::Cow(input.slice_from(input.position()).into()))
    }
}

impl std::fmt::Display for Atom<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl ToAtom for Atom<'_> {
    fn write_atom<W>(
        &self,
        dest: &mut crate::serialize::Printer<W>,
    ) -> Result<(), crate::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_str(self)
    }
}

#[cfg(feature = "markup5ever")]
impl From<string_cache::Atom<xml5ever::NamespaceStaticSet>> for Atom<'_> {
    fn from(value: string_cache::Atom<xml5ever::NamespaceStaticSet>) -> Self {
        Self::NS(value)
    }
}
#[cfg(feature = "markup5ever")]
impl From<string_cache::Atom<xml5ever::PrefixStaticSet>> for Atom<'_> {
    fn from(value: string_cache::Atom<xml5ever::PrefixStaticSet>) -> Self {
        Self::Prefix(value)
    }
}
#[cfg(feature = "markup5ever")]
impl From<string_cache::Atom<xml5ever::LocalNameStaticSet>> for Atom<'_> {
    fn from(value: string_cache::Atom<xml5ever::LocalNameStaticSet>) -> Self {
        Self::Local(value)
    }
}
#[cfg(feature = "markup5ever")]
impl From<tendril::StrTendril> for Atom<'_> {
    fn from(value: tendril::StrTendril) -> Self {
        Self::Tendril(value)
    }
}

impl<'input> Atom<'input> {
    pub fn push_str(&mut self, str: &str) {
        fn to_cow<'input>(a: &str, b: &str) -> Atom<'input> {
            let mut owned = String::with_capacity(a.len() + b.len());
            owned.push_str(a);
            owned.push_str(b);
            owned.into()
        }
        match self {
            Self::Cow(cow) => *self = to_cow(cow, str),
            Self::Static(string) => *self = to_cow(string, str),
            Self::NS(ns) => *self = to_cow(ns, str),
            Self::Prefix(prefix) => *self = to_cow(prefix, str),
            Self::Local(local) => *self = to_cow(local, str),
            Self::Tendril(tendril) => tendril.push_slice(str),
        }
    }

    // FIXME: remove when str_as_str is stable
    // https://github.com/rust-lang/rust/issues/130366
    pub fn as_str(&self) -> &str {
        self
    }
}
