use std::{cell::RefCell, collections::HashSet, fmt::Debug};

use itertools::Itertools;
use lightningcss::{
    declaration::DeclarationBlock,
    error::PrinterError,
    printer::{Printer, PrinterOptions},
    properties::Property,
    rules::{CssRule, CssRuleList},
    selector::{Component, Selector},
    traits::ToCss,
    values::{ident::Ident, string::CSSString},
    visit_types,
    visitor::{self, Visit, VisitTypes},
};
use oxvg_ast::{
    element::{Element, HashableElement},
    get_attribute, is_attribute, is_element,
    node::Node,
    remove_attribute, set_attribute,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::{atom::Atom, attribute::core::Style, is_prefix};
use oxvg_serialize::ToValue as _;
use parcel_selectors::{
    attr::{AttrSelectorOperator, CaseSensitivity},
    parser::LocalName,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::{error::JobsError, utils::minify_style};

#[derive(Debug, Clone)]
struct RemovedToken<'input, 'arena> {
    element: Element<'input, 'arena>,
    tokens: Vec<Token<'input>>,
    specificity: u32,
    declarations: DeclarationBlock<'input>,
}

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
/// Merges styles from a `<style>` element to the `style` attribute of matching elements.
///
/// # Differences to SVGO
///
/// Styles are minified via lightningcss when merged.
///
/// # Correctness
///
/// This job should never visually change the document.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct InlineStyles {
    /// If to only inline styles if the selector matches one element.
    #[serde(default = "default_only_matched_once")]
    pub only_matched_once: bool,
    /// If to remove the selector and styles from the stylesheet while inlining the styles. This
    /// does not remove the selectors that did not match any elements.
    #[serde(default = "default_remove_matched_selectors")]
    pub remove_matched_selectors: bool,
    /// An array of media query conditions to use, such as `screen`. An empty string signifies all
    /// selectors outside of a media query.
    /// Using `["*"]` will match all media-queries
    #[serde(default = "default_use_mqs")]
    pub use_mqs: Vec<String>,
    /// What pseudo-classes and pseudo-elements to use. An empty string signifies all non-pseudo
    /// classes and non-pseudo elements.
    /// Using `["*"]` will match all pseudo-elements and pseudo-classes.
    #[serde(default = "default_use_pseudos")]
    pub use_pseudos: Vec<String>,
}

#[allow(clippy::type_complexity)]
#[derive(Debug)]
struct State<'o, 'input, 'arena> {
    pub options: &'o InlineStyles,
    /// Which of matching tokens in a selector have been removed from a style element, and may be removed from the matching element too.
    pub inlined: RefCell<Vec<RemovedToken<'input, 'arena>>>,
    /// Which of matching tokens in a selector that are a dynamic reference.
    /// e.g. `.foo .bar` would record `.foo` as dynamic.
    pub dynamically_referenced: RefCell<HashSet<Token<'input>>>,
    /// Ids referenced in the document.
    pub referenced_ids: RefCell<HashSet<Atom<'input>>>,
}

#[derive(Debug)]
struct FindRemovableTokens<'o, 'input, 'arena> {
    /// A list specifying which media queries can be inlined
    options: &'o InlineStyles,
    /// Which tokens cannot be minified due to appearing as a parent token or within a preserved media query
    dynamically_referenced: HashSet<Token<'input>>,
    inlines: Vec<RemovedToken<'input, 'arena>>,
}

struct FindDynamicTokens<'a, 'o, 'input, 'arena> {
    find_removable_tokens: &'a mut FindRemovableTokens<'o, 'input, 'arena>,
    is_media_query: bool,
}

struct CollectMatchingSelectors<'a, 'o, 'input, 'arena> {
    find_removable_tokens: &'a mut FindRemovableTokens<'o, 'input, 'arena>,
    root: Element<'input, 'arena>,
}

struct InlinePresentationAttributes<'a, 'o, 'input, 'arena> {
    state: &'a State<'o, 'input, 'arena>,
    element: Element<'input, 'arena>,
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct AttrOperator(AttrSelectorOperator);

impl Debug for AttrOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AttrOperator")
            .field(&match self.0 {
                AttrSelectorOperator::Equal => "Equal",
                AttrSelectorOperator::Includes => "Includes",
                AttrSelectorOperator::DashMatch => "DashMatch",
                AttrSelectorOperator::Prefix => "Prefix",
                AttrSelectorOperator::Substring => "Substring",
                AttrSelectorOperator::Suffix => "Suffix",
            })
            .finish()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Token<'input> {
    Class {
        ident: Atom<'input>,
    },
    ID {
        ident: Atom<'input>,
    },
    Attr {
        local_name: Atom<'input>,
        value: Option<(Atom<'input>, AttrOperator, CaseSensitivity)>,
    },
    Name {
        local_name: Atom<'input>,
    },
    Other {
        token: String,
        is_preserved: bool,
    },
}

impl<'input> From<&Component<'input>> for Token<'input> {
    fn from(value: &Component<'input>) -> Self {
        match value {
            Component::Class(Ident(ident)) => Token::Class {
                ident: ident.clone().into(),
            },
            Component::ID(Ident(ident)) => Token::ID {
                ident: ident.clone().into(),
            },
            Component::LocalName(LocalName {
                name: Ident(name), ..
            }) => Token::Name {
                local_name: name.clone().into(),
            },
            Component::AttributeInNoNamespaceExists {
                local_name: Ident(name),
                ..
            } => Token::Attr {
                local_name: name.clone().into(),
                value: None,
            },
            Component::AttributeInNoNamespace {
                local_name: Ident(name),
                operator,
                value: CSSString(value),
                case_sensitivity,
                ..
            } => Token::Attr {
                local_name: name.clone().into(),
                value: Some((
                    value.clone().into(),
                    AttrOperator(*operator),
                    case_sensitivity.to_unconditional(false),
                )),
            },
            token => Token::Other {
                token: format!("{token:?}"),
                is_preserved: matches!(
                    token,
                    // FIX: Root often used for theming
                    // https://github.com/noahbald/oxvg/issues/153
                    // | Component::Root
                    Component::Is(_)
                        | Component::Negation(_)
                        | Component::Where(_)
                        | Component::Has(_)
                        | Component::Empty
                        | Component::Nth(_)
                        | Component::NthOf(_)
                ),
            },
        }
    }
}

impl Default for InlineStyles {
    fn default() -> Self {
        InlineStyles {
            only_matched_once: default_only_matched_once(),
            remove_matched_selectors: default_remove_matched_selectors(),
            use_mqs: default_use_mqs(),
            use_pseudos: default_use_pseudos(),
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for InlineStyles {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        State::new(self).start_with_context(document, context)?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for State<'_, 'input, 'arena> {
    type Error = JobsError<'input>;

    fn exit_element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut referenced_ids = self.referenced_ids.borrow_mut();
        for mut attr in element.attributes().into_iter_mut() {
            if is_attribute!(attr, Id) {
                continue;
            }
            let mut value = attr.value_mut();
            value.visit_id(|id| {
                referenced_ids.insert(id.to_string().into());
            });
            value.visit_url(|url| {
                if let Some(url) = url.strip_prefix('#') {
                    referenced_ids.insert(url.to_string().into());
                }
            });
        }
        if !is_element!(element, Style) {
            return Ok(());
        }

        if let Some(style_type) = get_attribute!(element, TypeStyle) {
            if !style_type.is_empty() && style_type.as_str() != "text/css" {
                log::debug!("Not merging style: unsupported type");
                return Ok(());
            }
        }

        let Some(css) = element.first_child().and_then(Node::style) else {
            log::debug!("Not merging style: empty");
            return Ok(());
        };

        if context.flags.contains(ContextFlags::within_foreign_object) {
            log::debug!("Not merging style: foreign-object");
            return Ok(());
        }

        let mut find_removable_tokens = FindRemovableTokens::new(self.options);
        let css = &mut *css.borrow_mut();
        if let Err(err) = find_removable_tokens.inline_rules(css, &context.root) {
            log::debug!("Not merging style: {err}");
            return Ok(());
        }
        self.dynamically_referenced
            .borrow_mut()
            .extend(find_removable_tokens.dynamically_referenced);
        self.inlined
            .borrow_mut()
            .extend(find_removable_tokens.inlines);
        minify_style::style_list(css).ok();
        if css.0.is_empty() {
            element.remove();
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn exit_document(
        &self,
        _root: &Element<'input, 'arena>,
        _context: &Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let dynamically_referenced = self.dynamically_referenced.borrow();
        let inlined = self.inlined.borrow();
        let referenced_ids = self.referenced_ids.borrow();
        #[allow(clippy::mutable_key_type)]
        let grouping = inlined
            .iter()
            .into_group_map_by(|RemovedToken { element, .. }| {
                HashableElement::new(element.clone())
            });
        for (element, mut group) in grouping {
            group.sort_by(|a, b| a.specificity.cmp(&b.specificity));

            let mut style_attr = remove_attribute!(element, Style)
                .unwrap_or_else(|| Style(DeclarationBlock::default()));
            let mut new_inline_style = DeclarationBlock::default();
            for RemovedToken { declarations, .. } in &group {
                new_inline_style
                    .declarations
                    .extend(declarations.declarations.clone());
                new_inline_style
                    .important_declarations
                    .extend(declarations.important_declarations.clone());
            }
            new_inline_style
                .declarations
                .extend(style_attr.declarations.clone());
            new_inline_style
                .important_declarations
                .extend(style_attr.important_declarations.clone());
            style_attr.0 = new_inline_style;

            style_attr.0.visit(&mut InlinePresentationAttributes {
                state: self,
                element: (*element).clone(),
            })?;
            if !style_attr.declarations.is_empty() || !style_attr.important_declarations.is_empty()
            {
                minify_style::style(&mut style_attr.0);
                set_attribute!(element, Style(style_attr));
            }

            for RemovedToken {
                tokens, element, ..
            } in group
            {
                for token in tokens {
                    if dynamically_referenced.contains(token) {
                        continue;
                    }
                    match token {
                        Token::Class { ident } => element.class_list().remove(ident),
                        Token::ID { ident } => {
                            if referenced_ids.contains(ident) {
                                continue;
                            }
                            remove_attribute!(element, Id);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'o, 'input, 'arena> FindRemovableTokens<'o, 'input, 'arena> {
    fn new(options: &'o InlineStyles) -> Self {
        Self {
            options,
            dynamically_referenced: HashSet::new(),
            inlines: Vec::new(),
        }
    }

    fn inline_rules(
        &mut self,
        stylesheet: &mut CssRuleList<'input>,
        root: &Element<'input, 'arena>,
    ) -> Result<(), anyhow::Error> {
        // First pass to find dynamic tokens, which will skip inlining.
        stylesheet.visit(self)?;
        // Second pass will take matching selectors from the stylesheet
        let mut collect_matching_selectors = CollectMatchingSelectors {
            find_removable_tokens: self,
            root: root.clone(),
        };
        stylesheet.visit(&mut collect_matching_selectors)?;

        Ok(())
    }
}

impl<'input, 'arena> CollectMatchingSelectors<'_, '_, 'input, 'arena> {
    fn strip_allowed_pseudos(&self, selector: String) -> String {
        let mut new_selector = None;
        for pseudo in &self.find_removable_tokens.options.use_pseudos {
            let Some(stripped) = new_selector
                .unwrap_or(selector.as_str())
                .strip_suffix(pseudo)
            else {
                continue;
            };
            new_selector = Some(stripped);
        }
        match new_selector {
            Some(s) => s.to_string(),
            None => selector,
        }
    }

    fn is_selector_removable(
        &mut self,
        selector: &mut Selector<'input>,
        matches: &[Element<'input, 'arena>],
    ) -> Option<Vec<Token<'input>>> {
        let options = self.find_removable_tokens.options;
        let use_any_pseudo = options.use_pseudos.first().is_some_and(|s| s == "*");

        if selector.has_pseudo_element() {
            log::debug!("selector has pseudo-element: {selector:?}");
            return Some(Vec::with_capacity(0));
        }
        if selector.has_combinator() {
            return None;
        }
        let simple_selector: Vec<_> = selector.iter().map(Token::from).collect();
        if !use_any_pseudo
            && !self.find_removable_tokens.options.use_pseudos.contains(
                &simple_selector
                    .iter()
                    .filter_map(|p| match p {
                        Token::Other {
                            token,
                            is_preserved: false,
                        } => Some(token),
                        _ => None,
                    })
                    .filter(|p| p.starts_with(':'))
                    .join(""),
            )
        {
            log::debug!("selector has disallowed pseudo: {simple_selector:?}");
            return None;
        }

        let match_count = matches
            .iter()
            .filter(|m| {
                simple_selector.iter().all(|token| match token {
                    Token::Class { ident } => m.has_class(ident),
                    Token::ID { ident } => get_attribute!(m, Id).is_some_and(|id| id.0 == *ident),
                    Token::Attr { local_name, value } => match value {
                        Some((value, operator, sensitivity)) => {
                            m.get_attribute_local(local_name).is_some_and(|atom| {
                                operator.0.eval_str(
                                    &atom
                                        .value()
                                        .to_value_string(PrinterOptions::default())
                                        .unwrap(),
                                    value,
                                    *sensitivity,
                                )
                            })
                        }
                        None => m.get_attribute_local(local_name).is_some(),
                    },
                    Token::Name { local_name } => m.local_name() == local_name,
                    Token::Other {
                        token,
                        is_preserved,
                    } => {
                        *is_preserved
                            || self
                                .find_removable_tokens
                                .options
                                .use_pseudos
                                .contains(token)
                    }
                })
            })
            .count();
        log::debug!("selector {simple_selector:?} has matches: {match_count:?}");

        let removable = if self.find_removable_tokens.options.only_matched_once {
            match_count == 1
        } else {
            match_count > 0
        };
        if removable {
            Some(simple_selector)
        } else {
            None
        }
    }
}

impl<'input> visitor::Visitor<'input> for CollectMatchingSelectors<'_, '_, 'input, '_> {
    type Error = PrinterError;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(RULES)
    }

    fn visit_rule(&mut self, rule: &mut CssRule<'input>) -> Result<(), Self::Error> {
        let CssRule::Style(style) = rule else {
            return rule.visit_children(self);
        };

        if !style.rules.0.is_empty() {
            // treat nested selectors as dynamic
            return Ok(());
        }
        let declarations = &style.declarations;

        style.selectors.0.retain(|selector| {
            let selector_iter = &mut selector.iter();
            let tail: Vec<_> = selector_iter.map(Token::from).collect();
            if tail.iter().any(|token| {
                self.find_removable_tokens
                    .dynamically_referenced
                    .contains(token)
            }) {
                self.find_removable_tokens
                    .dynamically_referenced
                    .extend(tail);
                while selector_iter.next_sequence().is_some() {
                    self.find_removable_tokens
                        .dynamically_referenced
                        .extend(selector_iter.map(Token::from));
                }
                log::debug!("retained selector: dynamic reference used");
                return true;
            }
            let Ok(selector_string) = selector.to_css_string(PrinterOptions {
                minify: true,
                ..PrinterOptions::default()
            }) else {
                return true;
            };
            let selector_string = self.strip_allowed_pseudos(selector_string);
            let Ok(matches) = self.root.select(&selector_string) else {
                log::debug!("retained selector: no matches");
                return true;
            };
            let matches: Vec<_> = matches.collect();
            let Some(matching_tokens) = self.is_selector_removable(selector, &matches) else {
                return true;
            };
            for m in matches {
                self.find_removable_tokens.inlines.push(RemovedToken {
                    element: m.clone(),
                    tokens: matching_tokens.clone(),
                    specificity: selector.specificity(),
                    declarations: declarations.clone(),
                });
            }
            false
        });
        if style.selectors.0.is_empty() {
            style.declarations = DeclarationBlock::default();
        }
        Ok(())
    }
}

impl<'input> visitor::Visitor<'input> for FindRemovableTokens<'_, 'input, '_> {
    type Error = PrinterError;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(RULES)
    }

    fn visit_rule(&mut self, rule: &mut CssRule<'input>) -> Result<(), Self::Error> {
        let use_mqs = &self.options.use_mqs;
        let mut find_dynamic_tokens = FindDynamicTokens {
            find_removable_tokens: self,
            is_media_query: false,
        };
        let options = PrinterOptions::default();
        let media_query = match rule {
            CssRule::Media(media) => {
                format!("media {}", media.query.to_css_string(options)?)
            }
            CssRule::Supports(supports) => {
                format!("supports {}", supports.condition.to_css_string(options)?)
            }
            CssRule::LayerBlock(layer) => match &layer.name {
                Some(name) => format!("layer {}", name.to_css_string(options)?),
                None => "layer".to_string(),
            },
            CssRule::Container(container) => match &container.name {
                Some(name) => format!("container {}", name.to_css_string(options)?),
                None => "container".to_string(),
            },
            CssRule::Scope(scope) => {
                let mut result = String::from("scope");
                let mut printer = Printer::new(&mut result, options);
                if let Some(scope_start) = &scope.scope_start {
                    printer.write_char('(')?;
                    scope_start.to_css(&mut printer)?;
                    printer.write_char(')')?;
                }
                if let Some(scope_end) = &scope.scope_end {
                    printer.write_str(" to (")?;
                    scope_end.to_css(&mut printer)?;
                    printer.write_char(')')?;
                }
                result
            }
            CssRule::StartingStyle(_) => "starting-style".to_string(),
            _ => {
                rule.visit(&mut find_dynamic_tokens)?;
                return Ok(());
            }
        };
        if use_mqs.contains(&media_query) {
            return Ok(());
        }

        // Mark full media query selectors as dynamic
        find_dynamic_tokens.is_media_query = true;
        rule.visit_children(&mut find_dynamic_tokens)
    }
}

impl<'input> visitor::Visitor<'input> for FindDynamicTokens<'_, '_, 'input, '_> {
    type Error = PrinterError;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(RULES | SELECTORS)
    }

    fn visit_selector(&mut self, selector: &mut Selector<'input>) -> Result<(), Self::Error> {
        let iter = &mut selector.iter();
        // Tail of selector, mark tokens as dynamic when in media query
        iter.for_each(|token| {
            if self.is_media_query {
                self.find_removable_tokens
                    .dynamically_referenced
                    .insert(token.into());
            }
        });
        // Combinators, mark tokens as dynamic
        while iter.next_sequence().is_some() {
            iter.for_each(|token| {
                self.find_removable_tokens
                    .dynamically_referenced
                    .insert(token.into());
            });
        }
        Ok(())
    }
}

impl<'input> visitor::Visitor<'input> for InlinePresentationAttributes<'_, '_, 'input, '_> {
    type Error = JobsError<'input>;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(PROPERTIES)
    }

    fn visit_property(&mut self, property: &mut Property<'input>) -> Result<(), Self::Error> {
        let id = property.property_id();
        let name = id.name();
        if self
            .state
            .dynamically_referenced
            .borrow()
            .iter()
            .filter_map(|token| match token {
                Token::Attr { local_name, .. } => Some(local_name),
                _ => None,
            })
            .any(|item| &**item == name)
        {
            return Ok(());
        }
        self.element
            .attributes()
            .retain(|attr| !is_prefix!(attr, SVG) || &**attr.local_name() != name);
        Ok(())
    }
}

impl<'o> State<'o, '_, '_> {
    pub fn new(options: &'o InlineStyles) -> Self {
        Self {
            options,
            inlined: RefCell::new(Vec::new()),
            dynamically_referenced: RefCell::new(HashSet::new()),
            referenced_ids: RefCell::new(HashSet::new()),
        }
    }
}

const fn default_only_matched_once() -> bool {
    true
}
const fn default_remove_matched_selectors() -> bool {
    true
}
fn default_use_mqs() -> Vec<String> {
    vec![String::new(), String::from("screen")]
}
fn default_use_pseudos() -> Vec<String> {
    vec![String::new()]
}

#[test]
#[allow(clippy::too_many_lines)]
fn inline_styles() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <rect width="100" height="100" class="st0"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <style>
        .st0{fill:blue;}
    </style>
    <rect width="100" height="100" class="st0"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" id="dark" viewBox="0 0 258.12 225.88">
    <!-- for https://github.com/svg/svgo/pull/592#issuecomment-266327016 -->
    <style>
        .cls-7 {
            only-cls-7: 1;
        }
        .cls-7,
        .cls-8 {
            cls-7-and-8: 1;
        }
    </style>

    <path class="cls-7"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Should apply a single style based on specificity and cascade -->
    <style>
        .st0{fill:blue;}
        .st1{fill:red; }
    </style>
    <rect width="100" height="100" class="st0 st1"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- Existing styles should be retained -->
    <style>
        .st1 {
            fill: red;
        }
        .st0 {
            color: blue;
        }
    </style>
    <rect width="100" height="100" class="st0 st1" style="color:yellow"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": { "onlyMatchedOnce": false } }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- allow selector with multiple matches when not onlyMatchedOnce -->
    <style>
        .red {
            fill: red;
        }
        .blue {
            fill: blue;
        }
    </style>
    <rect width="100" height="100" class="red blue"/>
    <rect width="100" height="100" class="blue red"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- important styles take precedence -->
    <style>
        .red {
            fill: red !important;
        }
        .blue {
            fill: blue;
        }
    </style>
    <rect width="100" height="100" class="blue red"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- important styles take precedence over inline styles -->
    <style>
        .red {
            fill: red !important;
        }
        .blue {
            fill: blue;
        }
    </style>
    <rect width="100" height="100" class="blue red" style="fill:yellow"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- important inline styles take precedence over important styles -->
    <style>
        .red {
            fill: red !important;
        }
        .blue {
            fill: blue;
        }
    </style>
    <rect width="100" height="100" class="blue red" style="fill:yellow !important"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- CDATA content is used -->
    <style>
        <![CDATA[
            .st0{fill:blue;}
        ]]>
    </style>
    <rect width="100" height="100" class="st0"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- dynamic pseudo-classes are not applied -->
    <style>
        .st0{fill:blue;}
        .st0:hover{stroke:red;}
    </style>
    <rect width="100" height="100" class="st0"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": { "usePseudos": [":hover"] } }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
    <!-- specified usePseudos are allows to be moved -->
    <style>
        .st0:hover{stroke:red;}
    </style>
    <rect width="100" height="100" class="st0"/>
</svg>"#
        ),
    )?);

    // NOTE: Test edited, import moved to below @charset, otherwise it's invalid
    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 81.285 81.285">
    <!-- retains at-rules -->
    <defs>
        <style>

            /* Simple Atrules */
            @charset 'UTF-8';

            @import url('https://fonts.googleapis.com/css?family=Roboto');

            @namespace svg url(http://www.w3.org/2000/svg);

            /* Atrules with block */
            @font-face {
                font-family: SomeFont;
                src: local("Some Font"), local("SomeFont"), url(SomeFont.ttf);
                font-weight: bold;
            }

            @viewport {
                    zoom: 0.8;
                min-zoom: 0.4;
                max-zoom: 0.9;
            }

            @keyframes identifier {
                  0% { top:  0; }
                 50% { top: 30px; left: 20px; }
                 50% { top: 10px; }
                100% { top:  0; }
            }


            /* Nested rules */
            @page :first {
                margin: 1in;
            }

            @supports (display: flex) {
                .module { display: flex; }
            }

            @document url('http://example.com/test.html') {
                rect {
                    stroke: red;
                }
            }


            .blue {
                fill: blue;
            }
    </style>
    </defs>
    <rect width="100" height="100" class="blue"/>
</svg>"#
        ),
    )?);

    // NOTE: the minified version of the query must be specified, not the original
    // NOTE: lightningcss removes empty at-rules
    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": { "useMqs": ["media only screen and (device-width >= 320px) and (device-width <= 480px) and (-webkit-device-pixel-ratio >= 2)"] } }"#,
        Some(
            r#"<svg id="test" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 81.285 81.285">
    <!-- allow movement of matching useMqs -->
    <defs>
        <style>
            @media only screen
            and (min-device-width: 320px)
            and (max-device-width: 480px)
            and (-webkit-min-device-pixel-ratio: 2) {

                .blue { fill: blue; }

            }
        </style>
    </defs>
    <rect width="100" height="100" class="blue"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg viewBox="0 0 24 24" version="1.1" xmlns="http://www.w3.org/2000/svg">
    <!-- ignores deprecated shadow-dom selectors -->
    <defs>
        <style type="text/css">
            html /deep/ [layout][horizontal], html /deep/ [layout][vertical] { display: flex; }
            html /deep/ [layout][horizontal][inline], html /deep/ [layout][vertical][inline] { display: inline-flex; }
            html /deep/ [layout][horizontal] { flex-direction: row; }
            html /deep/ [layout][horizontal][reverse] { flex-direction: row-reverse; }
            html /deep/ [layout][vertical] { flex-direction: column; }
            html /deep/ [layout][vertical][reverse] { flex-direction: column-reverse; }
            html /deep/ [layout][wrap] { flex-wrap: wrap; }
            html /deep/ [layout][wrap-reverse] { flex-wrap: wrap-reverse; }
            html /deep/ [flex] { flex: 1 1 0px; }
            html /deep/ [flex][auto] { flex: 1 1 auto; }
            html /deep/ [flex][none] { flex: 0 0 auto; }
            html /deep/ [flex][one] { flex: 1 1 0px; }
            html /deep/ [flex][two] { flex: 2 1 0px; }
            html /deep/ [flex][three] { flex: 3 1 0px; }
            html /deep/ [flex][four] { flex: 4 1 0px; }
            html /deep/ [flex][five] { flex: 5 1 0px; }
            html /deep/ [flex][six] { flex: 6 1 0px; }
            html /deep/ [flex][seven] { flex: 7 1 0px; }
            html /deep/ [flex][eight] { flex: 8 1 0px; }
            html /deep/ [flex][nine] { flex: 9 1 0px; }
            html /deep/ [flex][ten] { flex: 10 1 0px; }
            html /deep/ [flex][eleven] { flex: 11 1 0px; }
            html /deep/ [flex][twelve] { flex: 12 1 0px; }
            html /deep/ [layout][start] { align-items: flex-start; }
            html /deep/ [layout][center] { align-items: center; }
            html /deep/ [layout][end] { align-items: flex-end; }
            html /deep/ [layout][start-justified] { justify-content: flex-start; }
            html /deep/ [layout][center-justified] { justify-content: center; }
            html /deep/ [layout][end-justified] { justify-content: flex-end; }
            html /deep/ [layout][around-justified] { justify-content: space-around; }
            html /deep/ [layout][justified] { justify-content: space-between; }
            html /deep/ [self-start] { align-self: flex-start; }
            html /deep/ [self-center] { align-self: center; }
            html /deep/ [self-end] { align-self: flex-end; }
            html /deep/ [self-stretch] { align-self: stretch; }
            html /deep/ [block] { display: block; }
            html /deep/ [hidden] { display: none !important; }
            html /deep/ [relative] { position: relative; }
            html /deep/ [fit] { position: absolute; top: 0px; right: 0px; bottom: 0px; left: 0px; }
            body[fullbleed] { margin: 0px; height: 100vh; }
            html /deep/ [segment], html /deep/ segment { display: block; position: relative; box-sizing: border-box; margin: 1em 0.5em; padding: 1em; -webkit-box-shadow: rgba(0, 0, 0, 0.0980392) 0px 0px 0px 1px; box-shadow: rgba(0, 0, 0, 0.0980392) 0px 0px 0px 1px; border-top-left-radius: 5px; border-top-right-radius: 5px; border-bottom-right-radius: 5px; border-bottom-left-radius: 5px; background-color: white; }
            html /deep/ core-icon { display: inline-block; vertical-align: middle; background-repeat: no-repeat; }
            html /deep/ core-icon[size=""] { position: relative; }
        </style>
    </defs>
    <g id="airplanemode-on">
        <path d="M10.2,9"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": { "onlyMatchedOnce": false } }"#,
        Some(
            r##"<svg id="Ebene_1" data-name="Ebene 1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 222 57.28">
    <!-- ids and classes handled correctly -->
    <defs>
        <style>
            #id0 {
                stroke: red;
            }
            #id1 {
                stroke: red;
            }

            .cls-1 {
                fill: #37d0cd;
            }

            .cls-2{
                fill: #fff;
            }
        </style>
    </defs>
    <title>button</title>
    <rect id="id0" class="cls-1" width="222" height="57.28" rx="28.64" ry="28.64"/>
    <rect id="id1" class="cls-1" width="222" height="57.28" rx="28.64" ry="28.64"/>
    <path class="cls-2" d="M312.75,168.66A2.15,2.15,0,0,1,311.2,165L316,160l-4.8-5a2.15,2.15,0,1,1,3.1-3l6.21,6.49a2.15,2.15,0,0,1,0,3L314.31,168a2.14,2.14,0,0,1-1.56.67Zm0,0" transform="translate(-119 -131.36)"/>
    <circle class="cls-2" cx="33.5" cy="27.25" r="2.94"/>
    <circle class="cls-2" cx="162.5" cy="158.61" r="2.94" transform="translate(-181.03 61.15) rotate(-52.89)"/>
    <circle class="cls-2" cx="172.5" cy="158.61" r="2.94" transform="translate(-157.03 -75.67) rotate(-16.55)"/>
    <a href="#id1">id reference</a>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <!-- foreignObject elements ignored -->
    <foreignObject width="100%" height="100%">
        <style>div { color: red; }</style>
        <body xmlns="http://www.w3.org/1999/xhtml"><div>hello, world</div></body>
    </foreignObject>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": { "onlyMatchedOnce": true } }"#,
        Some(
            r#"<!-- Generator: Adobe Illustrator 21.1.0, SVG Export Plug-In . SVG Version: 6.00 Build 0)  -->
<svg version="1.1" id="Logo" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px"
	 viewBox="0 0 24 24" style="enable-background:new 0 0 24 24;" xml:space="preserve">
    <!-- multiple matches are unmoved -->
    <style type="text/css">
        .st0{fill:#D1DAE5;}
    </style>
    <g>
        <path class="st0" d="M16.9,12.3c0-0.1,0.1-0.2,0.1-0.3c0,0,0-0.1,0-0.1c0-0.1,0-0.2,0-0.2c0,0,0,0,0,0c0-0.1-0.1-0.2-0.2-0.3 c0,0,0-0.1,0-0.1c0,0,0,0,0,0c0,0,0,0,0,0l-3.5-3.5c-0.4-0.4-1-0.4-1.4,0s-0.4,1,0,1.4l1.8,1.8H7.5c-0.6,0-1,0.4-1,1s0.4,1,1,1h6.1 l-1.9,1.9c-0.4,0.4-0.4,1,0,1.4c0.2,0.2,0.5,0.3,0.7,0.3c0.3,0,0.5-0.1,0.7-0.3l3.6-3.6c0,0,0,0,0,0c0.1-0.1,0.2-0.2,0.2-0.3 c0,0,0,0,0,0C16.9,12.3,16.9,12.3,16.9,12.3z"/>
        <path class="st0" d="M12,0C5.4,0,0,5.4,0,12s5.4,12,12,12s12-5.4,12-12S18.6,0,12,0z M12,22C6.5,22,2,17.5,2,12S6.5,2,12,2 s10,4.5,10,10S17.5,22,12,22z"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": { "onlyMatchedOnce": true } }"#,
        Some(
            r#"<svg id="icon_time" data-name="icon time" xmlns="http://www.w3.org/2000/svg" width="51" height="51" viewBox="0 0 51 51">
    <!-- only single matches are moved (i.e. .cls-1) -->
    <defs>
        <style>
            .cls-1, .cls-2, .cls-3 {
                fill: #f5f5f5;
                stroke: gray;
            }

            .cls-1, .cls-2 {
                stroke-width: 1px;
            }

            .cls-2 {
                fill-rule: evenodd;
            }

            .cls-3 {
                stroke-width: 2px;
            }
        </style>
    </defs>
    <circle class="cls-1" cx="25.5" cy="25.5" r="25"/>
    <g>
        <path class="cls-2" d="M1098,2415a8,8,0,0,1,8,8v2h-16v-2A8,8,0,0,1,1098,2415Z" transform="translate(-1072.5 -2389.5)"/>
        <path id="Ellipse_14_copy" data-name="Ellipse 14 copy" class="cls-2" d="M1098,2415a8,8,0,0,0,8-8v-2h-16v2A8,8,0,0,0,1098,2415Z" transform="translate(-1072.5 -2389.5)"/>
        <path class="cls-2" d="M1089,2427v-1h18v1h-18Z" transform="translate(-1072.5 -2389.5)"/>
        <path id="Shape_10_copy" data-name="Shape 10 copy" class="cls-2" d="M1089,2404v-1h18v1h-18Z" transform="translate(-1072.5 -2389.5)"/>
        <circle id="Ellipse_13_copy" data-name="Ellipse 13 copy" class="cls-3" cx="25.5" cy="31.5" r="1"/>
        <circle id="Ellipse_13_copy_3" data-name="Ellipse 13 copy 3" class="cls-3" cx="28.5" cy="31.5" r="1"/>
        <circle id="Ellipse_13_copy_2" data-name="Ellipse 13 copy 2" class="cls-3" cx="22.5" cy="31.5" r="1"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg>
    <!-- elements with zany type attributes ignored -->
    <style type="text/invalid">
        .invalid { fill: red; }
    </style>
    <style type="text/css">
        .css { fill: green; }
    </style>
    <style type="">
        .empty { fill: blue; }
    </style>
    <rect x="0" y="0" width="100" height="100" class="invalid" />
    <rect x="0" y="0" width="100" height="100" class="css" />
    <rect x="0" y="0" width="100" height="100" class="empty" />
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1570.062" height="2730" viewBox="0 0 415.412 722.312">
    <!-- selectors matching two classes should be handled -->
    <style>
        .segment.minor {
        stroke-width: 1.5;
        stroke: #15c6aa;
        }
    </style>
    <g transform="translate(200.662 362.87)">
        <path d="M163.502-303.979h3.762" class="segment minor"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1570.062" height="2730" viewBox="0 0 415.412 722.312">
    <!-- selectors matching two classes should be handled -->
    <style>
        .segment.minor {
            stroke-width: 1.5;
        }
        .minor {
            stroke: #15c6aa;
        }
    </style>
    <g transform="translate(200.662 362.87)">
        <path d="M163.502-303.979h3.762" class="segment minor"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 35">
    <!-- empty selectors are dropped -->
    <style>
        .a {}
    </style>
    <g class="a">
        <circle class="b" cx="42.97" cy="24.92" r="1.14"/>
    </g>
</svg>
"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 269 349">
    <!-- remove overridden presentation attribute -->
    <style type="text/css">
        .a {
        fill: #059669;
        }
    </style>
    <path class="a" d="M191.5,324.1V355l9.6-31.6A77.49,77.49,0,0,1,191.5,324.1Z" fill="#059669" transform="translate(-57.17 -13.4)"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 50">
  <style>
    .a {
      stroke: red;
    }

    [stroke] + path {
      stroke: purple;
    }
  </style>
  <path class="a" d="M10 10h20" stroke="red"/>
  <path d="M10 20h20"/>
  <path d="M10 30h20" stroke="yellow"/>
  <path d="M10 40h20"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 35">
    <!-- don't remove the wrapping class if it's the parent of another selector -->
    <style>
        .a {}

        .a .b {
            fill: none;
            stroke: #000;
        }
    </style>
    <g class="a">
        <circle class="b" cx="42.97" cy="24.92" r="1.14"/>
        <path class="b" d="M26,31s11.91-1.31,15.86-5.64"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 50">
  <style>
    path:not([fill=blue]) {
      stroke: purple;
    }
  </style>
  <path fill="red" d="M5 5H10"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 50">
    <!-- unmatched pseudo-classes should do nothing -->
    <style>
        path:not([fill=red]) {
            stroke: purple;
        }
    </style>
    <path fill="red" d="M5 5H10"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "inlineStyles": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 50">
    <!-- preserved pseudo-classes aren't inlined -->
    <style>
        :root {
            background: #fff;
        }
    </style>
</svg>"#
        ),
    )?);

    Ok(())
}
