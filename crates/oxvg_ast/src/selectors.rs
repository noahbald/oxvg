//! Types used for selecting elements with css selectors.
use std::{
    hash::{DefaultHasher, Hash as _, Hasher},
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
    attribute::{Attr as _, Attributes as _},
    class_list::ClassList as _,
    element::{self},
    name::Name,
    node::{self, Node as _},
};

#[derive(Debug, Clone)]
/// Specifies parser types
pub struct SelectorImpl<A: Atom, P: Atom, LN: Atom, NS: Atom> {
    atom: PhantomData<A>,
    prefix: PhantomData<P>,
    name: PhantomData<LN>,
    namespace: PhantomData<NS>,
}

#[derive(Eq, PartialEq, Debug, Clone, Default)]
/// A value
pub struct CssAtom<A: Atom>(pub A);

#[derive(Eq, PartialEq, Clone, Default)]
/// A local name or prefix
pub struct CssName<N: Atom>(pub N);

#[derive(Eq, PartialEq, Clone, Default)]
/// A namespace url
pub struct CssNamespace<NS: Atom>(pub NS);

#[derive(Eq, PartialEq, Clone)]
/// The type for a pseudo-class.
pub enum PseudoClass<A: Atom, P: Atom, LN: Atom, NS: Atom> {
    /// :any-link
    AnyLink(
        PhantomData<A>,
        PhantomData<P>,
        PhantomData<LN>,
        PhantomData<NS>,
    ),
    /// :link
    Link(
        PhantomData<A>,
        PhantomData<P>,
        PhantomData<LN>,
        PhantomData<NS>,
    ),
}

#[derive(Eq, PartialEq, Clone)]
/// The type for a pseudo-element.
pub struct PseudoElement<A: Atom, P: Atom, LN: Atom, NS: Atom> {
    atom: PhantomData<A>,
    prefix: PhantomData<P>,
    name: PhantomData<LN>,
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

impl<N: Atom> ToCss for CssName<N> {
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

impl<N: Atom> From<&str> for CssName<N> {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl<A: Atom, P: Atom, LN: Atom, NS: Atom> ToCss for PseudoClass<A, P, LN, NS> {
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

impl<N: Atom> PrecomputedHash for CssName<N> {
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

impl<A: Atom, P: Atom, LN: Atom, NS: Atom> selectors::parser::NonTSPseudoClass
    for PseudoClass<A, P, LN, NS>
{
    type Impl = SelectorImpl<A, P, LN, NS>;

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

impl<A: Atom, P: Atom, LN: Atom, NS: Atom> ToCss for PseudoElement<A, P, LN, NS> {
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

impl<A: Atom, P: Atom, LN: Atom, NS: Atom> selectors::parser::PseudoElement
    for PseudoElement<A, P, LN, NS>
{
    type Impl = SelectorImpl<A, P, LN, NS>;
}

impl<A: Atom, P: Atom, LN: Atom, NS: Atom> selectors::SelectorImpl for SelectorImpl<A, P, LN, NS> {
    type AttrValue = CssAtom<A>;
    type Identifier = CssName<LN>;
    type LocalName = CssName<LN>;
    type NamespacePrefix = CssName<P>;
    type NamespaceUrl = CssNamespace<NS>;
    type BorrowedNamespaceUrl = CssNamespace<NS>;
    type BorrowedLocalName = CssName<LN>;

    type NonTSPseudoClass = PseudoClass<A, P, LN, NS>;
    type PseudoElement = PseudoElement<A, P, LN, NS>;

    type ExtraMatchingData<'a> = ();
}

/// An iterator for the elements matching a given selector.
#[allow(clippy::type_complexity)]
pub struct Select<'arena, E: element::Element<'arena>> {
    inner: element::Iterator<'arena, E>,
    scope: Option<E>,
    selector: Selector<
        E::Atom,
        <E::Name as Name>::Prefix,
        <E::Name as Name>::LocalName,
        <E::Name as Name>::Namespace,
    >,
    selector_caches: SelectorCaches,
}

#[derive(Debug)]
/// A parsed selector.
pub struct Selector<A: Atom, P: Atom, LN: Atom, NS: Atom>(
    selectors::parser::SelectorList<SelectorImpl<A, P, LN, NS>>,
);

/// A parser for selectors.
pub struct Parser<A: Atom, P: Atom, LN: Atom, NS: Atom> {
    marker: PhantomData<(A, P, LN, NS)>,
}

impl<'arena, E: element::Element<'arena>> Select<'arena, E> {
    /// Creates an iterator over the elements matching the selector.
    ///
    /// # Errors
    /// If the selector fails to parse
    pub fn new<'a>(
        element: &'a E,
        selector: &'a str,
    ) -> Result<
        Select<'arena, E>,
        cssparser::ParseError<'a, selectors::parser::SelectorParseErrorKind<'a>>,
    > {
        Ok(Self::new_with_selector(
            element,
            Selector::new::<E>(selector)?,
        ))
    }

    /// Creates an iterator over the elements matching the selector, using the given selector.
    #[allow(clippy::type_complexity)]
    pub fn new_with_selector(
        element: &E,
        selector: Selector<
            E::Atom,
            <E::Name as Name>::Prefix,
            <E::Name as Name>::LocalName,
            <E::Name as Name>::Namespace,
        >,
    ) -> Select<'arena, E> {
        Select {
            inner: element.breadth_first(),
            scope: Some(element.clone()),
            selector,
            selector_caches: SelectorCaches::default(),
        }
    }
}

impl<'arena, E: element::Element<'arena>> Iterator for Select<'arena, E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find(|element| {
            element::Element::parent_element(element).is_some()
                && self.selector.matches_with_scope_and_cache(
                    &SelectElement {
                        element: element.clone(),
                        marker: PhantomData,
                    },
                    self.scope.clone(),
                    &mut self.selector_caches,
                )
        })
    }
}

impl<A: Atom, P: Atom, LN: Atom, NS: Atom> Selector<A, P, LN, NS> {
    /// # Errors
    /// If the selector fails to parse
    pub fn new<
        'arena,
        E: element::Element<'arena, Atom = A, Name: Name<Prefix = P, LocalName = LN, Namespace = NS>>,
    >(
        selector: &str,
    ) -> Result<Selector<A, P, LN, NS>, cssparser::ParseError<'_, SelectorParseErrorKind<'_>>> {
        let parser_input = &mut cssparser::ParserInput::new(selector);
        let parser = &mut cssparser::Parser::new(parser_input);

        let list = SelectorList::parse(&new_parser::<E>(), parser, ParseRelative::No)?;
        Ok(Selector(list))
    }
}

impl<A: Atom, P: Atom, LN: Atom, NS: Atom> Selector<A, P, LN, NS> {
    /// Returns whether the selector matches an element.
    pub fn matches_with_scope_and_cache<
        'arena,
        E: element::Element<'arena, Atom = A, Name: Name<Prefix = P, LocalName = LN, Namespace = NS>>,
    >(
        &self,
        element: &SelectElement<'arena, E>,
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
        context.scope_element = scope.map(|e| selectors::Element::opaque(&SelectElement::new(e)));
        matching::matches_selector_list(&self.0, element, context)
    }

    /// Returns whether the selector matches an element.
    pub fn matches_naive<
        'arena,
        E: element::Element<'arena, Atom = A, Name: Name<Prefix = P, LocalName = LN, Namespace = NS>>,
    >(
        &self,
        element: &SelectElement<'arena, E>,
    ) -> bool {
        self.matches_with_scope_and_cache(element, None, &mut SelectorCaches::default())
    }
}

impl<'i, A: Atom, P: Atom, LN: Atom, NS: Atom> selectors::parser::Parser<'i>
    for Parser<A, P, LN, NS>
{
    type Impl = SelectorImpl<A, P, LN, NS>;
    type Error = SelectorParseErrorKind<'i>;
}

#[allow(clippy::type_complexity)]
fn new_parser<'arena, E: element::Element<'arena>>() -> Parser<
    E::Atom,
    <E::Name as Name>::Prefix,
    <E::Name as Name>::LocalName,
    <E::Name as Name>::Namespace,
> {
    Parser {
        marker: PhantomData,
    }
}

#[derive(Debug, Clone)]
/// A wrapper for [`element::Element`] implementing [`selectors::Element`]
pub struct SelectElement<'arena, E: element::Element<'arena>> {
    element: E,
    marker: PhantomData<&'arena ()>,
}

impl<'arena, E: element::Element<'arena>> SelectElement<'arena, E> {
    /// Creates a selectable element using the given element
    pub fn new(element: E) -> Self {
        Self {
            element,
            marker: PhantomData,
        }
    }
}

impl<'arena, E: element::Element<'arena>> selectors::Element for SelectElement<'arena, E> {
    type Impl = SelectorImpl<
        <E as node::Node<'arena>>::Atom,
        <<E as node::Node<'arena>>::Name as Name>::Prefix,
        <<E as node::Node<'arena>>::Name as Name>::LocalName,
        <<E as node::Node<'arena>>::Name as Name>::Namespace,
    >;

    fn opaque(&self) -> selectors::OpaqueElement {
        selectors::OpaqueElement::new(self)
    }

    fn parent_element(&self) -> Option<Self> {
        self.element.parent_element().map(Self::new)
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.element.previous_element_sibling().map(Self::new)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.element.next_element_sibling().map(Self::new)
    }

    fn first_element_child(&self) -> Option<Self> {
        self.element.first_element_child().map(Self::new)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        true
    }

    fn has_local_name(
        &self,
        local_name: &<Self::Impl as selectors::SelectorImpl>::BorrowedLocalName,
    ) -> bool {
        if self.element.node_type() == node::Type::Document {
            false
        } else {
            self.element.local_name() == &local_name.0
        }
    }

    fn has_namespace(
        &self,
        ns: &<Self::Impl as selectors::SelectorImpl>::BorrowedNamespaceUrl,
    ) -> bool {
        self.element.qual_name().ns() == &ns.0
    }

    fn is_same_type(&self, other: &Self) -> bool {
        let name = self.element.qual_name();
        let other_name = other.element.qual_name();

        name.local_name() == other.element.local_name() && name.prefix() == other_name.prefix()
    }

    fn attr_matches(
        &self,
        ns: &selectors::attr::NamespaceConstraint<
            &<Self::Impl as selectors::SelectorImpl>::NamespaceUrl,
        >,
        local_name: &<Self::Impl as selectors::SelectorImpl>::LocalName,
        operation: &selectors::attr::AttrSelectorOperation<
            &<Self::Impl as selectors::SelectorImpl>::AttrValue,
        >,
    ) -> bool {
        use selectors::attr::NamespaceConstraint;

        let value = match ns {
            NamespaceConstraint::Any => self.element.get_attribute_local(&local_name.0),
            NamespaceConstraint::Specific(ns) => {
                self.element.get_attribute_ns(&ns.0, &local_name.0)
            }
        };
        let Some(value) = value else {
            return false;
        };
        operation.eval_str(&value)
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &<Self::Impl as selectors::SelectorImpl>::NonTSPseudoClass,
        _context: &mut matching::MatchingContext<Self::Impl>,
    ) -> bool {
        match pc {
            PseudoClass::Link(..) | PseudoClass::AnyLink(..) => self.is_link(),
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &<Self::Impl as selectors::SelectorImpl>::PseudoElement,
        _context: &mut matching::MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, flags: matching::ElementSelectorFlags) {
        let self_flags = flags.for_self();
        self.element.set_selector_flags(self_flags);

        let Some(parent) = self.element.parent_element() else {
            return;
        };
        let parent_flags = flags.for_parent();
        parent.set_selector_flags(parent_flags);
    }

    fn is_link(&self) -> bool {
        if self.element.node_type() == node::Type::Document {
            return false;
        }
        matches!(self.element.local_name().as_ref(), "a" | "area" | "link")
            && self.element.has_attribute_local(&"href".into())
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        if self.element.node_type() == node::Type::Document {
            return false;
        }
        let Some(self_id) = self.element.get_attribute_local(&"id".into()) else {
            return false;
        };
        case_sensitivity.eq(id.0.as_bytes(), self_id.as_bytes())
    }

    fn has_class(
        &self,
        name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        if self.element.node_type() == node::Type::Document {
            return false;
        }

        let Some(self_class) = self.element.get_attribute_local(&"class".into()) else {
            return false;
        };
        let name = name.0.as_bytes();
        self_class
            .split_whitespace()
            .any(|c| case_sensitivity.eq(name, c.as_bytes()))
    }

    fn imported_part(
        &self,
        _name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
    ) -> Option<<Self::Impl as selectors::SelectorImpl>::Identifier> {
        None
    }

    fn is_part(&self, _name: &<Self::Impl as selectors::SelectorImpl>::Identifier) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        !self.element.has_child_nodes()
            || self.element.child_nodes_iter().all(|child| {
                child.node_type() == node::Type::Text
                    && child
                        .text_content()
                        .is_none_or(|string| string.trim().is_empty())
            })
    }

    fn is_root(&self) -> bool {
        self.element.is_root()
    }

    fn has_custom_state(
        &self,
        _name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
    ) -> bool {
        false
    }

    #[allow(clippy::cast_possible_truncation)]
    fn add_element_unique_hashes(&self, filter: &mut selectors::bloom::BloomFilter) -> bool {
        let mut f = |hash: u32| filter.insert_hash(hash & selectors::bloom::BLOOM_HASH_MASK);

        let local_name_hash = &mut DefaultHasher::default();
        self.element.local_name().hash(local_name_hash);
        f(local_name_hash.finish() as u32);

        let prefix_hash = &mut DefaultHasher::default();
        self.element.prefix().hash(prefix_hash);
        f(prefix_hash.finish() as u32);

        if let Some(id) = self.element.get_attribute_local(&"id".into()) {
            let id_hash = &mut DefaultHasher::default();
            id.hash(id_hash);
            f(prefix_hash.finish() as u32);
        }

        for class in self.element.class_list().iter() {
            let class_hash = &mut DefaultHasher::default();
            class.hash(class_hash);
            f(class_hash.finish() as u32);
        }

        for attr in self.element.attributes().into_iter() {
            let name = attr.name();
            if matches!(name.local_name().as_ref(), "class" | "id" | "style") {
                continue;
            }

            let name_hash = &mut DefaultHasher::default();
            name.hash(name_hash);
            f(name_hash.finish() as u32);
        }
        true
    }
}
