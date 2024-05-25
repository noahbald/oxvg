use std::{
    borrow::BorrowMut,
    cell::{Cell, RefCell},
    rc::Rc,
};

use cssparser::ToCss;
use derivative::Derivative;
use markup5ever::{local_name, Attribute, LocalName, Namespace};
use rcdom::NodeData;
use selectors::{
    attr::{CaseSensitivity, NamespaceConstraint},
    matching::{self, ElementSelectorFlags},
    parser::{ParseRelative, SelectorParseErrorKind},
    NthIndexCache, SelectorList,
};

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Element {
    pub node: RcNode,
    #[derivative(Debug = "ignore")]
    selector_flags: Cell<Option<ElementSelectorFlags>>,
}

pub struct Select {
    inner: RefCell<Vec<RcNode>>,
    selector: Selector,
    nth_index_cache: NthIndexCache,
}

pub struct Selector(SelectorList<SelectorImpl>);

pub struct Parser;

#[derive(Eq, PartialEq, Clone, Default)]
pub struct CssLocalName(LocalName);

pub struct Attributes<'a>(pub &'a Vec<Attribute>);

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct AttributeValue(String);

#[derive(Debug, Clone)]
pub struct SelectorImpl;

#[derive(Eq, PartialEq, Clone)]
pub enum PseudoClass {
    AnyLink,
    Link,
}

#[derive(Eq, PartialEq, Clone)]
pub enum PseudoElement {}

type RcNode = Rc<rcdom::Node>;

fn is_element(child: &RcNode) -> bool {
    matches!(child.data, NodeData::Element { .. })
}

fn eq_elements(other: &RcNode) -> Box<dyn Fn(&RcNode) -> bool + '_> {
    Box::new(|child: &RcNode| Rc::ptr_eq(child, other))
}

impl Element {
    pub fn new(node: RcNode) -> Self {
        Self {
            node,
            selector_flags: Cell::new(None),
        }
    }

    pub fn from_document_root(document: &rcdom::RcDom) -> Option<Self> {
        Some(Self::new(
            document.document.children.borrow().first()?.to_owned(),
        ))
    }

    // FIXME: Collecting for these 'siblings' functions seems redundant
    // but I can't figure out how to fix the return of temporary value
    // when collecting is removed.
    // Maybe we make a macro later?
    fn siblings(&self) -> Option<Vec<RcNode>> {
        use selectors::Element;

        Some(
            self.parent_element()?
                .node
                .children
                .borrow()
                .clone()
                .into_iter()
                .filter(is_element)
                .collect(),
        )
    }

    fn preceding_siblings(&self) -> Option<Vec<RcNode>> {
        let mut preceding_siblings = self
            .siblings()?
            .into_iter()
            .rev()
            .skip_while(eq_elements(&self.node));
        preceding_siblings.next();
        Some(preceding_siblings.collect())
    }

    fn following_siblings(&self) -> Option<Vec<RcNode>> {
        let mut preceding_siblings = self
            .siblings()?
            .into_iter()
            .skip_while(eq_elements(&self.node));
        preceding_siblings.next();
        Some(preceding_siblings.collect())
    }

    pub fn set_selector_flags(&self, selector_flags: ElementSelectorFlags) {
        if selector_flags.is_empty() {
            return;
        };
        self.selector_flags.set(Some(
            selector_flags | self.selector_flags.take().unwrap_or(selector_flags),
        ));
    }

    pub fn get_attr(&self, attr: &markup5ever::LocalName) -> Option<Attribute> {
        let NodeData::Element { ref attrs, .. } = self.node.as_ref().data else {
            return None;
        };
        Attributes(&attrs.borrow()).get_attr(attr)
    }

    pub fn get_attr_as_number<F: std::str::FromStr>(
        &self,
        attr: &LocalName,
    ) -> Option<Result<F, F::Err>> {
        self.get_attr(attr).map(|attr| attr.value.parse())
    }

    /// # Errors
    /// If the selector is invalid
    pub fn select<'a>(
        &'a self,
        selector: &'a str,
    ) -> Result<Select, cssparser::ParseError<'_, SelectorParseErrorKind<'_>>> {
        Ok(Select {
            inner: self.node.children.clone(),
            selector: selector.try_into()?,
            nth_index_cache: NthIndexCache::default(),
        })
    }
}

impl selectors::Element for Element {
    type Impl = SelectorImpl;
    fn opaque(&self) -> selectors::OpaqueElement {
        selectors::OpaqueElement::new(&self.node)
    }

    fn parent_element(&self) -> Option<Self> {
        let parent = self.node.parent.take()?;
        let parent_element = parent.upgrade().map(Self::from);
        self.node.parent.set(Some(parent));
        parent_element
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
        self.preceding_siblings()?.first().map(Self::from)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.following_siblings()?.first().map(Self::from)
    }

    fn first_element_child(&self) -> Option<Self> {
        self.node
            .children
            .borrow()
            .clone()
            .into_iter()
            .find(is_element)
            .map(Self::from)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        true
    }

    fn has_local_name(
        &self,
        local_name: &<Self::Impl as selectors::SelectorImpl>::BorrowedLocalName,
    ) -> bool {
        let NodeData::Element { ref name, .. } = self.node.as_ref().data else {
            return false;
        };
        name.local == local_name.0
    }

    fn has_namespace(
        &self,
        ns: &<Self::Impl as selectors::SelectorImpl>::BorrowedNamespaceUrl,
    ) -> bool {
        let NodeData::Element { ref name, .. } = self.node.as_ref().data else {
            return false;
        };
        &name.ns == ns
    }

    fn is_same_type(&self, other: &Self) -> bool {
        let NodeData::Element { ref name, .. } = self.node.as_ref().data else {
            return false;
        };
        let NodeData::Element {
            name: ref other_name,
            ..
        } = other.node.as_ref().data
        else {
            return false;
        };
        name.local == other_name.local && name.prefix == other_name.prefix
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
        let NodeData::Element { ref attrs, .. } = self.node.as_ref().data else {
            return false;
        };
        let attrs = attrs.borrow();
        match ns {
            NamespaceConstraint::Any => attrs.iter().any(|attr| {
                attr.name.local == local_name.0 && operation.eval_str(attr.value.as_ref())
            }),
            NamespaceConstraint::Specific(ns) => attrs.iter().any(|attr| {
                &&attr.name.ns == ns
                    && attr.name.local == local_name.0
                    && operation.eval_str(attr.value.as_ref())
            }),
        }
    }

    fn match_non_ts_pseudo_class(
        &self,
        pc: &<Self::Impl as selectors::SelectorImpl>::NonTSPseudoClass,
        _context: &mut selectors::matching::MatchingContext<Self::Impl>,
    ) -> bool {
        match pc {
            PseudoClass::Link | PseudoClass::AnyLink => self.is_link(),
        }
    }

    fn match_pseudo_element(
        &self,
        pe: &<Self::Impl as selectors::SelectorImpl>::PseudoElement,
        _context: &mut selectors::matching::MatchingContext<Self::Impl>,
    ) -> bool {
        match *pe {}
    }

    fn apply_selector_flags(&self, flags: selectors::matching::ElementSelectorFlags) {
        let self_flags = flags.for_self();
        self.set_selector_flags(self_flags);

        let Some(parent) = self.parent_element() else {
            return;
        };
        let parent_flags = flags.for_parent();
        parent.set_selector_flags(parent_flags);
    }

    fn is_link(&self) -> bool {
        let NodeData::Element {
            ref name,
            ref attrs,
            ..
        } = self.node.as_ref().data
        else {
            return false;
        };
        matches!(
            name.local,
            local_name!("a") | local_name!("area") | local_name!("link")
        ) && attrs
            .borrow()
            .iter()
            .any(|attr| matches!(attr.name.local, local_name!("href")))
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: selectors::attr::CaseSensitivity,
    ) -> bool {
        let Some(self_id) = self.get_attr(&local_name!("id")) else {
            return false;
        };
        case_sensitivity.eq(id.0.as_bytes(), self_id.value.as_bytes())
    }

    fn has_class(
        &self,
        name: &<Self::Impl as selectors::SelectorImpl>::Identifier,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        let Some(self_class) = self.get_attr(&local_name!("class")) else {
            return false;
        };
        case_sensitivity.eq(name.0.as_bytes(), self_class.value.as_bytes())
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
        self.node
            .children
            .borrow()
            .iter()
            .all(|child| match &child.data {
                NodeData::Element { .. } => false,
                NodeData::Text { contents } => contents.borrow().is_empty(),
                _ => true,
            })
    }

    fn is_root(&self) -> bool {
        let Some(parent) = self.node.parent.take() else {
            return false;
        };
        let Some(mut parent_node) = parent.upgrade() else {
            return false;
        };
        self.node.parent.set(Some(parent));
        matches!(parent_node.borrow_mut().data, NodeData::Document)
    }
}

impl From<&mut RcNode> for Element {
    fn from(value: &mut RcNode) -> Self {
        Self {
            node: value.clone(),
            selector_flags: Cell::new(None),
        }
    }
}

impl From<&RcNode> for Element {
    fn from(value: &RcNode) -> Self {
        Self {
            node: value.clone(),
            selector_flags: Cell::new(None),
        }
    }
}

impl From<RcNode> for Element {
    fn from(value: RcNode) -> Self {
        Self {
            node: value,
            selector_flags: Cell::new(None),
        }
    }
}

impl selectors::SelectorImpl for SelectorImpl {
    type AttrValue = AttributeValue;
    type Identifier = CssLocalName;
    type LocalName = CssLocalName;
    type NamespacePrefix = CssLocalName;
    type NamespaceUrl = Namespace;
    type BorrowedNamespaceUrl = Namespace;
    type BorrowedLocalName = CssLocalName;

    type NonTSPseudoClass = PseudoClass;
    type PseudoElement = PseudoElement;

    type ExtraMatchingData<'a> = ();
}

impl<'a> From<&'a str> for AttributeValue {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

impl AsRef<str> for AttributeValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ToCss for AttributeValue {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        cssparser::serialize_string(&self.0, dest)
    }
}

impl<'a> From<&'a str> for CssLocalName {
    fn from(value: &'a str) -> Self {
        Self(value.into())
    }
}

impl ToCss for CssLocalName {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        dest.write_str(&self.0)
    }
}

impl selectors::parser::PseudoElement for PseudoElement {
    type Impl = SelectorImpl;
}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        dest.write_str(&self.to_css_string())
    }

    fn to_css_string(&self) -> String {
        match *self {}
    }
}

impl selectors::parser::NonTSPseudoClass for PseudoClass {
    type Impl = SelectorImpl;

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

impl ToCss for PseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        dest.write_str(&self.to_css_string())
    }

    fn to_css_string(&self) -> String {
        match self {
            Self::Link => ":link",
            Self::AnyLink => ":any-link",
        }
        .into()
    }
}

impl Iterator for Select {
    type Item = Element;

    fn next(&mut self) -> Option<Self::Item> {
        use selectors::Element as _;

        for node in self.inner.borrow().clone() {
            if !is_element(&node) {
                continue;
            }
            let element = Element::new(node);
            if element.parent_element().is_some()
                && self.selector.matches_with_scope_and_cache(
                    &element,
                    None,
                    &mut self.nth_index_cache,
                )
            {
                return Some(element);
            }
        }
        None
    }
}

impl Selector {
    fn matches_with_scope_and_cache(
        &self,
        element: &Element,
        scope: Option<Element>,
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
}

impl<'a> TryFrom<&'a str> for Selector {
    type Error = cssparser::ParseError<'a, SelectorParseErrorKind<'a>>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let parser_input = &mut cssparser::ParserInput::new(value);
        let parser = &mut cssparser::Parser::new(parser_input);

        SelectorList::parse(&Parser, parser, ParseRelative::No).map(Self)
    }
}

impl<'i> selectors::parser::Parser<'i> for Parser {
    type Impl = SelectorImpl;
    type Error = SelectorParseErrorKind<'i>;
}

impl<'a> Attributes<'a> {
    pub fn get_attr(&self, attr: &LocalName) -> Option<Attribute> {
        self.0
            .iter()
            .find(|&self_attr| &self_attr.name.local == attr)
            .map(std::borrow::ToOwned::to_owned)
    }
}
