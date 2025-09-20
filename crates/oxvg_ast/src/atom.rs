//! String type for parsed values
use std::ops::Deref;

use lightningcss::values::string::CowArcStr;
#[cfg(feature = "markup5ever")]
use tendril::SliceExt;

use crate::{parse::Parse, serialize::ToAtom};

#[derive(Debug, Clone)]
/// Represents a constant value found while parsing the document.
///
/// # Features
///
/// By default atoms will reference the source document.
///
/// Certain parsers (e.g. `feature = "markup5ever"`) will enable
/// atomised values for common SVG strings and interning otherwise.
pub enum Atom<'input> {
    /// The string is an atom generated in a `const` or `static` context
    Static(&'static str),
    /// The string is a string generated while parsing
    Cow(CowArcStr<'input>),
    #[cfg(feature = "markup5ever")]
    /// The string is an atomised namespace
    NS(string_cache::Atom<xml5ever::NamespaceStaticSet>),
    #[cfg(feature = "markup5ever")]
    /// The string is an atomised prefix
    Prefix(string_cache::Atom<xml5ever::PrefixStaticSet>),
    #[cfg(feature = "markup5ever")]
    /// The string is an atomised local name
    Local(string_cache::Atom<xml5ever::LocalNameStaticSet>),
    #[cfg(feature = "markup5ever")]
    /// The string is an interned string generated while parsing
    Tendril(tendril::StrTendril),
}

impl Atom<'_> {
    /// Returns the atom into a value with a static lifetime, converting to a static variant if necessary.
    pub fn into_owned<'any>(self) -> Atom<'any>
    where
        'static: 'any,
    {
        match self {
            Self::Static(str) => str.into(),
            Self::Cow(cow) => cow.to_string().into(),
            #[cfg(feature = "markup5ever")]
            Self::NS(ns) => ns.into(),
            #[cfg(feature = "markup5ever")]
            Self::Prefix(prefix) => prefix.into(),
            #[cfg(feature = "markup5ever")]
            Self::Local(local) => local.into(),
            #[cfg(feature = "markup5ever")]
            Self::Tendril(tendril) => tendril.into(),
        }
    }
}

impl Default for Atom<'_> {
    fn default() -> Self {
        Self::Static("")
    }
}
impl std::hash::Hash for Atom<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
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
        **self == **other
    }
}

impl Ord for Atom<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}
impl PartialOrd for Atom<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'input> From<&'input str> for Atom<'input> {
    fn from(value: &'input str) -> Self {
        Self::Cow(value.into())
    }
}

impl From<String> for Atom<'static> {
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
        let start = input.position();
        while input.next().is_ok() {}
        Ok(Self::Cow(input.slice_from(start).into()))
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
impl From<string_cache::Atom<xml5ever::NamespaceStaticSet>> for Atom<'static> {
    fn from(value: string_cache::Atom<xml5ever::NamespaceStaticSet>) -> Self {
        Self::NS(value)
    }
}
#[cfg(feature = "markup5ever")]
impl From<string_cache::Atom<xml5ever::PrefixStaticSet>> for Atom<'static> {
    fn from(value: string_cache::Atom<xml5ever::PrefixStaticSet>) -> Self {
        Self::Prefix(value)
    }
}
#[cfg(feature = "markup5ever")]
impl From<string_cache::Atom<xml5ever::LocalNameStaticSet>> for Atom<'static> {
    fn from(value: string_cache::Atom<xml5ever::LocalNameStaticSet>) -> Self {
        Self::Local(value)
    }
}
#[cfg(feature = "markup5ever")]
impl From<tendril::StrTendril> for Atom<'static> {
    fn from(value: tendril::StrTendril) -> Self {
        Self::Tendril(value)
    }
}

impl Atom<'_> {
    /// Appends a given string slice onto the end of this `Atom`
    pub fn push_str(&mut self, str: &str) {
        fn to_cow<'input>(a: &str, b: &str) -> Atom<'input> {
            let mut owned = String::with_capacity(a.len() + b.len());
            owned.push_str(a);
            owned.push_str(b);
            owned.into()
        }
        #[cfg(feature = "markup5ever")]
        fn from_tendril<'input>(mut a: tendril::StrTendril, b: &str) -> Atom<'input> {
            a.push_slice(b);
            a.into()
        }
        match self {
            Self::Cow(cow) => *self = to_cow(cow, str),
            Self::Static(string) => *self = to_cow(string, str),
            #[cfg(feature = "markup5ever")]
            Self::NS(ns) => *self = from_tendril(ns.to_tendril(), str),
            #[cfg(feature = "markup5ever")]
            Self::Prefix(prefix) => *self = from_tendril(prefix.to_tendril(), str),
            #[cfg(feature = "markup5ever")]
            Self::Local(local) => *self = from_tendril(local.to_tendril(), str),
            #[cfg(feature = "markup5ever")]
            Self::Tendril(tendril) => tendril.push_slice(str),
        }
    }

    // FIXME: remove when str_as_str is stable
    // https://github.com/rust-lang/rust/issues/130366
    /// Extracts a string slice containing the entire atom
    pub fn as_str(&self) -> &str {
        self
    }
}
