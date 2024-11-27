use std::marker::PhantomData;

use cssparser::ToCss;
use selectors::{
    matching,
    parser::{ParseRelative, SelectorParseErrorKind},
    NthIndexCache, SelectorList,
};

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

pub struct ElementIterator<E: crate::element::Element> {
    current: E,
    index_cache: Vec<usize>,
}

pub struct Select<E: crate::element::Element> {
    inner: ElementIterator<E>,
    scope: Option<E>,
    selector: Selector<E>,
    nth_index_cache: selectors::NthIndexCache,
}

pub struct Selector<E: crate::element::Element>(selectors::parser::SelectorList<E::Impl>);

pub struct Parser<E: crate::element::Element> {
    element: PhantomData<E>,
}

impl<E: crate::element::Element> ElementIterator<E> {
    /// Returns a depth-first iterator starting at the given element
    pub fn new(element: &E) -> Self {
        Self {
            current: element.to_owned(),
            index_cache: Vec::default(),
        }
    }

    fn get_first_child(&mut self) -> Option<<Self as Iterator>::Item> {
        let child = crate::element::Element::first_element_child(&self.current)?;
        self.index_cache.push(0);
        Some(child)
    }

    fn get_next_sibling(&mut self) -> Option<<Self as Iterator>::Item> {
        let self_index = self.index_cache.pop()?;
        let parent = crate::element::Element::parent_element(&self.current)?;
        let mut siblings = parent.children_iter().skip(self_index);
        debug_assert!(
            siblings
                .next()
                .expect("Parent children no longer fits node")
                .ptr_eq(&self.current),
            "Parent children no longer holds node in place"
        );
        let next_element = siblings.next();
        if let Some(next_element) = next_element {
            self.index_cache.push(self_index + 1);
            self.current = next_element.clone();
            Some(next_element)
        } else {
            self.current = parent.clone();
            self.get_next_sibling()
        }
    }
}

impl<E: crate::element::Element> Iterator for ElementIterator<E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.get_first_child();
        if let Some(result) = result {
            self.current = result.clone();
            return Some(result);
        }

        let result = self.get_next_sibling()?;
        self.current = result.clone();
        Some(result)
    }
}

impl<E: crate::element::Element> Select<E> {
    /// # Errors
    /// If the selector fails to parse
    pub fn new<'a>(
        element: &'a E,
        selector: &'a str,
    ) -> Result<Select<E>, cssparser::ParseError<'a, selectors::parser::SelectorParseErrorKind<'a>>>
    {
        Ok(crate::selectors::Select {
            inner: element.depth_first(),
            scope: Some(element.clone()),
            selector: Selector::new(selector)?,
            nth_index_cache: NthIndexCache::default(),
        })
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
                    &mut self.nth_index_cache,
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
        nth_index_cache: &mut NthIndexCache,
    ) -> bool {
        let context = &mut matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            nth_index_cache,
            matching::QuirksMode::NoQuirks,
            matching::NeedsSelectorFlags::No,
            matching::IgnoreNthChildForInvalidation::No,
        );
        context.scope_element = scope.map(|x| selectors::Element::opaque(&x));
        self.0
             .0
            .iter()
            .any(|s| matching::matches_selector(s, 0, None, element, context))
    }

    pub fn matches_naive(&self, element: &E) -> bool {
        self.matches_with_scope_and_cache(element, None, &mut NthIndexCache::default())
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
