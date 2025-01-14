use std::{
    hash::{DefaultHasher, Hasher},
    marker::PhantomData,
};

use cssparser::ToCss;
use precomputed_hash::PrecomputedHash;
use selectors::{
    context::SelectorCaches,
    matching,
    parser::{ParseRelative, SelectorParseErrorKind},
    SelectorList,
};

use crate::{
    atom::Atom,
    element::{self},
};

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
        cssparser::serialize_string(self.0.as_ref(), dest)
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
        cssparser::serialize_string(self.0.as_ref(), dest)
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

impl<N: Atom> PrecomputedHash for CssLocalName<N> {
    #[allow(clippy::cast_possible_truncation)] // fine for hash
    fn precomputed_hash(&self) -> u32 {
        let mut output = DefaultHasher::default();
        self.0.hash(&mut output);
        output.finish() as u32
    }
}

impl<N: Atom> PrecomputedHash for CssNamespace<N> {
    #[allow(clippy::cast_possible_truncation)] // fine for hash
    fn precomputed_hash(&self) -> u32 {
        let mut output = DefaultHasher::default();
        self.0.hash(&mut output);
        output.finish() as u32
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

pub struct Select<E: crate::element::Element> {
    inner: element::Iterator<E>,
    scope: Option<E>,
    selector: Selector<E>,
    selector_caches: SelectorCaches,
}

#[derive(Debug)]
pub struct Selector<E: crate::element::Element>(selectors::parser::SelectorList<E::Impl>);

pub struct Parser<E: crate::element::Element> {
    element: PhantomData<E>,
}

impl<E: crate::element::Element> Select<E> {
    /// # Errors
    /// If the selector fails to parse
    pub fn new<'a>(
        element: &'a E,
        selector: &'a str,
    ) -> Result<Select<E>, cssparser::ParseError<'a, selectors::parser::SelectorParseErrorKind<'a>>>
    {
        Ok(Self::new_with_selector(element, Selector::new(selector)?))
    }

    pub fn new_with_selector(element: &E, selector: Selector<E>) -> Select<E> {
        crate::selectors::Select {
            inner: element.breadth_first(),
            scope: Some(element.clone()),
            selector,
            selector_caches: SelectorCaches::default(),
        }
    }
}

impl<E: crate::element::Element> Iterator for Select<E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find(|element| {
            crate::element::Element::parent_element(element).is_some()
                && self.selector.matches_with_scope_and_cache(
                    element,
                    self.scope.clone(),
                    &mut self.selector_caches,
                )
        })
    }
}

impl<E: crate::element::Element> Selector<E> {
    /// # Errors
    /// If the selector fails to parse
    pub fn new(
        selector: &str,
    ) -> Result<Self, cssparser::ParseError<'_, SelectorParseErrorKind<'_>>> {
        let parser_input = &mut cssparser::ParserInput::new(selector);
        let parser = &mut cssparser::Parser::new(parser_input);

        SelectorList::parse(&Parser::<E>::default(), parser, ParseRelative::No).map(Self)
    }
}

impl<E: crate::element::Element> Selector<E> {
    pub fn matches_with_scope_and_cache(
        &self,
        element: &E,
        scope: Option<E>,
        selector_caches: &mut SelectorCaches,
    ) -> bool {
        let context = &mut matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            selector_caches,
            matching::QuirksMode::NoQuirks,
            matching::NeedsSelectorFlags::No,
            matching::MatchingForInvalidation::No,
        );
        context.scope_element = scope.map(|x| selectors::Element::opaque(&x));
        matching::matches_selector_list(&self.0, element, context)
    }

    pub fn matches_naive(&self, element: &E) -> bool {
        self.matches_with_scope_and_cache(element, None, &mut SelectorCaches::default())
    }
}

impl<'i, E: crate::element::Element> selectors::parser::Parser<'i> for Parser<E> {
    type Impl = E::Impl;
    type Error = SelectorParseErrorKind<'i>;
}

impl<E: crate::element::Element> Default for Parser<E> {
    fn default() -> Self {
        Self {
            element: PhantomData,
        }
    }
}
