use std::marker::PhantomData;

use cssparser::ToCss;

use crate::atom::Atom;

#[derive(Debug, Clone)]
pub struct SelectorImpl<A: Atom, N: Atom, NS: Atom> {
    atom: PhantomData<A>,
    name: PhantomData<N>,
    namespace: PhantomData<NS>,
}

#[derive(Eq, PartialEq, Debug, Clone, Default)]
pub struct CssAtom<A: Atom>(pub A);

#[derive(Eq, PartialEq, Clone, Default)]
pub struct CssLocalName<N: Atom>(pub N);

#[derive(Eq, PartialEq, Clone, Default)]
pub struct CssNamespace<NS: Atom>(pub NS);

#[derive(Eq, PartialEq, Clone)]
pub enum PseudoClass<A: Atom, N: Atom, NS: Atom> {
    AnyLink(PhantomData<A>, PhantomData<N>, PhantomData<NS>),
    Link(PhantomData<A>, PhantomData<N>, PhantomData<NS>),
}

#[derive(Eq, PartialEq, Clone)]
pub struct PseudoElement<A: Atom, N: Atom, NS: Atom> {
    atom: PhantomData<A>,
    name: PhantomData<N>,
    namespace: PhantomData<NS>,
}

impl<A: Atom> ToCss for CssAtom<A> {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        cssparser::serialize_string(&self.0.clone().into(), dest)
    }
}

impl<A: Atom> AsRef<str> for CssAtom<A> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<N: Atom> ToCss for CssLocalName<N> {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        cssparser::serialize_string(&self.0.clone().into(), dest)
    }
}

impl<A: Atom> From<&str> for CssAtom<A> {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl<N: Atom> From<&str> for CssLocalName<N> {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl<A: Atom, N: Atom, NS: Atom> ToCss for PseudoClass<A, N, NS> {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        dest.write_str(&self.to_css_string())
    }

    fn to_css_string(&self) -> String {
        match self {
            Self::Link(..) => ":link",
            Self::AnyLink(..) => ":any-link",
        }
        .into()
    }
}

impl<A: Atom, N: Atom, NS: Atom> selectors::parser::NonTSPseudoClass for PseudoClass<A, N, NS> {
    type Impl = SelectorImpl<A, N, NS>;

    fn is_active_or_hover(&self) -> bool {
        false
    }

    fn is_user_action_state(&self) -> bool {
        false
    }

    fn visit<V>(&self, _visitor: &mut V) -> bool
    where
        V: selectors::visitor::SelectorVisitor<Impl = Self::Impl>,
    {
        false
    }
}

impl<A: Atom, N: Atom, NS: Atom> ToCss for PseudoElement<A, N, NS> {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        dest.write_str(&self.to_css_string())
    }

    fn to_css_string(&self) -> String {
        String::default()
    }
}

impl<A: Atom, N: Atom, NS: Atom> selectors::parser::PseudoElement for PseudoElement<A, N, NS> {
    type Impl = SelectorImpl<A, N, NS>;
}

impl<A: Atom, N: Atom, NS: Atom> selectors::SelectorImpl for SelectorImpl<A, N, NS> {
    type AttrValue = CssAtom<A>;
    type Identifier = CssLocalName<N>;
    type LocalName = CssLocalName<N>;
    type NamespacePrefix = CssLocalName<N>;
    type NamespaceUrl = CssNamespace<NS>;
    type BorrowedNamespaceUrl = CssNamespace<NS>;
    type BorrowedLocalName = CssLocalName<N>;

    type NonTSPseudoClass = PseudoClass<A, N, NS>;
    type PseudoElement = PseudoElement<A, N, NS>;

    type ExtraMatchingData<'a> = ();
}
