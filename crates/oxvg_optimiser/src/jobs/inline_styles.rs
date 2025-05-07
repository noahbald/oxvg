use std::{cell::RefCell, collections::BTreeSet, marker::PhantomData};

use derive_where::derive_where;
use itertools::Itertools;
use lightningcss::{media_query, printer, rules, selector, stylesheet, traits::ToCss};
use oxvg_ast::{
    atom::Atom,
    attribute::Attr,
    class_list::ClassList,
    element::Element,
    name::Name,
    visitor::{Context, ContextFlags, PrepareOutcome, Visitor},
};
use oxvg_collections::collections::{PRESENTATION, PSEUDO_FUNCTIONAL, PSEUDO_TREE_STRUCTURAL};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

#[derive(Debug)]
pub(crate) struct CapturedStyles<'arena, E: Element<'arena>> {
    node: E,
    css: String,
    marker: PhantomData<&'arena ()>,
}

#[derive(Debug)]
#[derive_where(Clone)]
struct RemovedToken<'arena, E: Element<'arena>> {
    element: E,
    token: Vec<E::Atom>,
    specificity: u32,
    declarations: String,
}

#[derive(Clone, Debug)]
#[derive_where(Default)]
pub(crate) struct RemovedTokens<'arena, E: Element<'arena>> {
    classes: Vec<RemovedToken<'arena, E>>,
    ids: Vec<RemovedToken<'arena, E>>,
    other: Vec<RemovedToken<'arena, E>>,
}

#[derive(Default, Debug, Clone)]
pub struct ParentTokens {
    presentation_attrs: BTreeSet<String>,
    classes: BTreeSet<String>,
    ids: BTreeSet<String>,
}

pub(crate) struct State<'o, 'arena, E: Element<'arena>> {
    pub options: &'o InlineStyles,
    pub styles: RefCell<Vec<CapturedStyles<'arena, E>>>,
    pub removed_tokens: RefCell<RemovedTokens<'arena, E>>,
    /// After running, a record of matching tokens in a selector that are an ancestor of a matching
    /// element.
    /// e.g. `.foo .bar` would record `.foo` as a parent token.
    pub parent_tokens: RefCell<ParentTokens>,
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
    /// Using `["*"]` will match all pseudo-elements
    #[serde(default = "default_use_pseudos")]
    pub use_pseudos: Vec<String>,
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

enum Token {
    Class,
    ID,
    Attr,
    Other,
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for InlineStyles {
    type Error = String;

    fn prepare(
        &self,
        document: &E,
        info: &oxvg_ast::visitor::Info<'arena, E>,
        _context_flags: &mut ContextFlags,
    ) -> Result<PrepareOutcome, Self::Error> {
        State::new(self).start(&mut document.clone(), info, None)?;
        Ok(PrepareOutcome::skip)
    }
}

impl<'arena, E: Element<'arena>> Visitor<'arena, E> for State<'_, 'arena, E> {
    type Error = String;

    fn exit_element(
        &self,
        element: &mut E,
        context: &mut Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        if element.prefix().is_some() || element.local_name().as_ref() != "style" {
            return Ok(());
        }

        if let Some(style_type) = element.get_attribute_local(&"type".into()) {
            if !style_type.is_empty() && style_type.as_ref() != "text/css" {
                log::debug!("Not merging style: unsupported type");
                return Ok(());
            }
        }

        let Some(css) = element.text_content() else {
            log::debug!("Not merging style: empty");
            return Ok(());
        };
        let parse_options = stylesheet::ParserOptions {
            flags: stylesheet::ParserFlags::all(),
            ..stylesheet::ParserOptions::default()
        };
        let mut css = match stylesheet::StyleSheet::parse(&css, parse_options) {
            Ok(css) => css,
            Err(e) => {
                log::debug!("Not merging style: {e}");
                return Ok(());
            }
        };

        if context.flags.contains(ContextFlags::within_foreign_object) {
            if let Ok(css) = css.rules.to_css_string(printer::PrinterOptions {
                minify: true,
                ..printer::PrinterOptions::default()
            }) {
                element
                    .clone()
                    .set_text_content(css.into(), &context.info.arena);
            }
            log::debug!("Not merging style: foreign-object");
            return Ok(());
        }

        let matches_styles = self
            .options
            .take_matching_selectors(&mut css.rules, context, self);
        if let Ok(css_string) = css.rules.to_css_string(printer::PrinterOptions {
            minify: true,
            ..printer::PrinterOptions::default()
        }) {
            self.styles.borrow_mut().push(CapturedStyles {
                node: element.clone(),
                css: css_string,
                marker: PhantomData,
            });
        }
        let Some(removed_styles) = matches_styles else {
            log::debug!("no styles moved");
            return Ok(());
        };
        let removed_styles = flatten_media(removed_styles);

        let new_removed_tokens = self.gather_removed_tokens(&removed_styles, context);
        let mut removed_tokens = self.removed_tokens.borrow_mut();
        removed_tokens.classes.extend(new_removed_tokens.classes);
        removed_tokens.ids.extend(new_removed_tokens.ids);
        removed_tokens.other.extend(new_removed_tokens.other);

        // For any styles that didn't match, keep them in `<style>`
        let css: String = css
            .rules
            .to_css_string(stylesheet::PrinterOptions {
                minify: true,
                ..stylesheet::PrinterOptions::default()
            })
            .expect("Unhandled error after updating inline styles");
        if css.is_empty() {
            log::debug!("all styles removed from element");
            element.remove();
        } else {
            element
                .clone()
                .set_text_content(css.into(), &context.info.arena);
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn exit_document(
        &self,
        _root: &mut E,
        _context: &Context<'arena, '_, '_, E>,
    ) -> Result<(), Self::Error> {
        let removed_tokens = self.removed_tokens.borrow();
        let parent_tokens = self.parent_tokens.borrow();
        removed_tokens
            .classes
            .iter()
            .for_each(|RemovedToken { element, token, .. }| {
                let mut class_list = element.class_list();
                for token in token {
                    if parent_tokens.classes.contains(token.as_str()) {
                        continue;
                    }
                    class_list.remove(token);
                }
            });
        let id_name = &"id".into();
        removed_tokens
            .ids
            .iter()
            .for_each(|RemovedToken { element, token, .. }| {
                if token.iter().any(|t| parent_tokens.ids.contains(t.as_str())) {
                    return;
                }
                element.remove_attribute_local(id_name);
            });

        // declarations, sorted by specificity, grouped by element
        let style_chunks = removed_tokens
            .ids
            .iter()
            .chain(removed_tokens.classes.iter())
            .chain(removed_tokens.other.iter())
            .sorted_by(|a, b| a.specificity.cmp(&b.specificity))
            .sorted_by(|a, b| a.element.id().cmp(&b.element.id()))
            .chunk_by(|r| r.element.id());

        for (_, chunk) in &style_chunks {
            let mut group_element = None;
            let mut style = String::new();
            chunk.into_iter().for_each(
                |RemovedToken {
                     element,
                     declarations,
                     ..
                 }| {
                    group_element = Some(element);
                    style.push_str(declarations.as_str());
                    style.push(';');
                },
            );
            let group_element = group_element.expect("chunks shouldn't be empty");
            let style_name = "style".into();
            let original_inline_styles = group_element
                .get_attribute_local(&style_name)
                .map(|a| a.to_string())
                .unwrap_or_default();
            style.push_str(&original_inline_styles);
            let mut css = match stylesheet::StyleAttribute::parse(
                &style,
                stylesheet::ParserOptions::default(),
            ) {
                Ok(css) => css,
                Err(e) => {
                    log::warn!("failed to move styles for element: {e}");
                    continue;
                }
            };
            css.minify(stylesheet::MinifyOptions::default());
            let printer_options = printer::PrinterOptions {
                minify: true,
                ..printer::PrinterOptions::default()
            };
            let Ok(style) = css.declarations.to_css_string(printer_options) else {
                log::warn!("failed to move styles for element");
                continue;
            };
            if style.is_empty() {
                group_element.remove_attribute_local(&style_name);
            } else {
                group_element.set_attribute_local(style_name, style.into());
            }

            css.declarations
                .declarations
                .iter()
                .chain(css.declarations.important_declarations.iter())
                .for_each(|f| {
                    let id = f.property_id();
                    let name = id.name();
                    if !PRESENTATION.contains(name) {
                        return;
                    }
                    if parent_tokens.presentation_attrs.iter().any(|s| s == name) {
                        return;
                    }
                    let name = <E::Attr as Attr>::Name::parse(name);
                    group_element.remove_attribute(&name);
                });
        }
        Ok(())
    }
}

impl<'o, 'arena, E: Element<'arena>> State<'o, 'arena, E> {
    pub fn new(options: &'o InlineStyles) -> Self {
        Self {
            options,
            styles: RefCell::new(vec![]),
            removed_tokens: RefCell::new(RemovedTokens::default()),
            parent_tokens: RefCell::new(ParentTokens::default()),
        }
    }

    fn gather_removed_tokens(
        &self,
        styles: &rules::CssRuleList,
        context: &Context<'arena, '_, '_, E>,
    ) -> RemovedTokens<'arena, E> {
        let mut removed_classes = vec![];
        let mut removed_ids = vec![];
        let mut removed_others = vec![];

        styles.0.iter().for_each(|rule| match rule {
            rules::CssRule::Media(media_rule) => {
                self.gather_removed_tokens(&media_rule.rules, context);
            }
            rules::CssRule::Style(ref style_rule) => {
                let mut selector = format!("{}", style_rule.selectors);
                selector = self.options.strip_allowed_pseudos(selector);
                let declarations =
                    match style_rule
                        .declarations
                        .to_css_string(printer::PrinterOptions {
                            minify: true,
                            ..printer::PrinterOptions::default()
                        }) {
                        Ok(d) => d,
                        Err(e) => {
                            log::debug!("couldn't move unparseable declarations: {e:?}");
                            return;
                        }
                    };
                let selected: Vec<_> = match context.root.select(selector.as_str()) {
                    Ok(i) => i,
                    Err(e) => {
                        log::debug!(r#"couldn't move invalid selector "{selector}": {e:?}"#);
                        return;
                    }
                }
                .collect();
                if let Some(r) =
                    Self::find_removed_classes(selected.iter(), style_rule, &declarations)
                {
                    removed_classes.extend(r);
                } else if let Some(r) =
                    Self::find_removed_ids(selected.iter(), style_rule, &declarations)
                {
                    removed_ids.extend(r);
                } else if let Some(r) =
                    Self::find_removed_others(selected.iter(), style_rule, &declarations)
                {
                    removed_others.extend(r);
                }
            }
            _ => unreachable!(),
        });
        RemovedTokens {
            classes: removed_classes,
            ids: removed_ids,
            other: removed_others,
        }
    }

    fn find_removed_classes<'a>(
        selected: impl Iterator<Item = &'a E>,
        style_rule: &rules::style::StyleRule,
        declarations: &str,
    ) -> Option<Vec<RemovedToken<'arena, E>>>
    where
        E: 'a,
    {
        let matching_classes: Vec<(Vec<E::Atom>, u32)> = style_rule
            .selectors
            .0
            .iter()
            .map(|s| {
                let matching_tokens: Vec<_> = s
                    .iter()
                    .take_while(|t| matches!(t, selector::Component::Class(_)))
                    .collect();
                let specificity = s.specificity();
                (matching_tokens, specificity)
            })
            .filter(|(t, _)| !t.is_empty())
            .map(|(t, s)| {
                (
                    t.into_iter()
                        .map(|t| format!("{t:?}")[1..].into())
                        .collect(),
                    s,
                )
            })
            .collect();

        let selected = selected.filter_map(|m| {
            let class_list = m.class_list();
            if class_list.length() == 0 {
                return None;
            }
            let (class, specificity) = matching_classes
                .iter()
                .find(|c| c.0.iter().all(|c| class_list.contains(c)))?;
            Some(RemovedToken {
                element: m.clone(),
                token: class.clone(),
                specificity: *specificity,
                declarations: declarations.to_string(),
            })
        });
        let output: Vec<_> = selected.collect();
        if output.is_empty() {
            None
        } else {
            Some(output)
        }
    }

    fn find_removed_ids<'a>(
        selected: impl Iterator<Item = &'a E>,
        style_rule: &rules::style::StyleRule,
        declarations: &str,
    ) -> Option<Vec<RemovedToken<'arena, E>>>
    where
        E: 'a,
    {
        let matching_ids: Vec<(E::Atom, u32)> = style_rule
            .selectors
            .0
            .iter()
            .filter_map(|s| {
                let last_token = s.iter().last()?;
                let specificity = s.specificity();
                Some((last_token, specificity))
            })
            .filter(|(t, _)| matches!(t, selector::Component::ID(_)))
            .map(|(t, s)| (format!("{t:?}")[1..].into(), s))
            .collect();

        let id_name = &"id".into();
        let selected = selected.filter_map(|m| {
            let id = m.get_attribute_local(id_name)?;
            let (id, specificity) = matching_ids
                .iter()
                .find(|(t, _)| t.as_ref() == id.as_ref())?;
            Some(RemovedToken {
                element: m.clone(),
                token: vec![id.clone()],
                specificity: *specificity,
                declarations: declarations.to_string(),
            })
        });
        let output: Vec<_> = selected.collect();
        if output.is_empty() {
            None
        } else {
            Some(output)
        }
    }

    fn find_removed_others<'a>(
        selected: impl Iterator<Item = &'a E>,
        style_rule: &rules::style::StyleRule,
        declarations: &str,
    ) -> Option<Vec<RemovedToken<'arena, E>>>
    where
        E: 'a,
    {
        #[allow(clippy::redundant_closure_for_method_calls)]
        let matching: Vec<u32> = style_rule
            .selectors
            .0
            .iter()
            .filter(|s| {
                !matches!(
                    s.iter().last(),
                    Some(selector::Component::ID(_) | selector::Component::Class(_))
                )
            })
            .map(|s| s.specificity())
            .collect();

        let output: Vec<_> = selected
            .flat_map(|m| {
                matching.iter().map(|s| RemovedToken {
                    element: m.clone(),
                    token: vec![],
                    specificity: *s,
                    declarations: declarations.to_string(),
                })
            })
            .collect();
        if output.is_empty() {
            None
        } else {
            Some(output)
        }
    }
}

fn flatten_media(css: rules::CssRuleList) -> rules::CssRuleList {
    rules::CssRuleList(
        css.0
            .into_iter()
            .flat_map(|rule| {
                let rules::CssRule::Media(rules::media::MediaRule { rules, .. }) = rule else {
                    return vec![rule];
                };
                rules.0
            })
            .collect(),
    )
}

impl InlineStyles {
    pub(crate) fn take_matching_selectors<'arena, 'a, 'o, E: Element<'arena>>(
        &self,
        css: &mut rules::CssRuleList<'a>,
        context: &Context<'arena, '_, '_, E>,
        state: &State<'o, 'arena, E>,
    ) -> Option<rules::CssRuleList<'a>> {
        let mut removed = rules::CssRuleList(vec![]);
        let mut matching_elements = vec![];

        css.0.retain_mut(|rule| match rule {
            rules::CssRule::Media(media_rule) => {
                let query = &media_rule.query;
                let debug_query = query.to_css_string(printer::PrinterOptions::default());
                if !self.is_media_query_useable(query) {
                    log::debug!("media query not useable: {debug_query:?}");
                    return true;
                }
                let Some(removed_media) =
                    self.take_matching_selectors(&mut media_rule.rules, context, state)
                else {
                    log::debug!("nothing removed from query: {debug_query:?}");
                    return true;
                };
                removed
                    .0
                    .push(rules::CssRule::Media(rules::media::MediaRule {
                        query: query.clone(),
                        rules: removed_media,
                        loc: media_rule.loc,
                    }));
                !media_rule.rules.0.is_empty()
            }
            #[allow(clippy::default_trait_access)]
            rules::CssRule::Style(style_rule) => {
                let mut parent_tokens = state.parent_tokens.borrow_mut();
                let mut removed_selector = style_rule.clone();
                let mut selector = format!("{}", style_rule.selectors);
                selector = self.strip_allowed_pseudos(selector);
                let matches = match context
                    .root
                    .select(selector.as_str())
                    .map(std::iter::Iterator::collect::<Vec<_>>)
                {
                    Ok(matches) => matches,
                    Err(e) => {
                        log::debug!("{e:?}: {selector}");
                        return true;
                    }
                };
                let new_parent_tokens = find_parent_attrs(style_rule);
                parent_tokens.classes.extend(new_parent_tokens.classes);
                parent_tokens.ids.extend(new_parent_tokens.ids);
                if !new_parent_tokens.presentation_attrs.is_empty() {
                    parent_tokens
                        .presentation_attrs
                        .extend(new_parent_tokens.presentation_attrs);
                    log::debug!("selector has presentation attr");
                    return true;
                }

                removed_selector.selectors.0 = Default::default();
                style_rule.selectors.0.retain_mut(|s| {
                    if self.is_selector_removeable(s, &mut removed_selector, &matches) {
                        !self.remove_matched_selectors
                    } else {
                        true
                    }
                });
                removed.0.push(rules::CssRule::Style(removed_selector));
                matching_elements.extend(matches);
                if style_rule.selectors.0.is_empty() {
                    !self.remove_matched_selectors
                } else {
                    true
                }
            }
            _ => true,
        });
        if removed.0.is_empty() {
            None
        } else {
            Some(removed)
        }
    }

    fn is_media_query_useable(&self, media_rule: &media_query::MediaList) -> bool {
        if self.use_mqs.is_empty() {
            return false;
        }
        if self.use_mqs.first().expect("previously checked is_empty") == "*" {
            return true;
        }
        let media_query = match media_rule.to_css_string(printer::PrinterOptions::default()) {
            Ok(media_query) => media_query,
            Err(e) => {
                log::debug!("failed to parse media_rule: {e}");
                return false;
            }
        };
        self.use_mqs.contains(&media_query)
    }

    fn is_selector_removeable<'arena, 'a, E: Element<'arena>>(
        &self,
        selector: &mut selector::Selector<'a>,
        removed_selector: &mut rules::style::StyleRule<'a>,
        matches: &[E],
    ) -> bool {
        let use_any_pseudo = self.use_pseudos.first().is_some_and(|s| s == "*");

        if selector.has_pseudo_element() {
            log::debug!("selector has pseudo-element: {selector:?}");
            return true;
        }
        if selector
            .iter()
            .filter(|p| !matches!(p, selector::Component::NonTSPseudoClass(_)))
            .map(|p| format!("{p:?}"))
            .filter(|p| use_any_pseudo || self.use_pseudos.contains(p))
            .any(|p| PRESERVED_PSEUDOS.contains(p.as_str()))
        {
            log::debug!("selector has pseudo-class: {selector:?}");
            return true;
        }

        let Some(mut token) = selector
            .iter()
            .map(|p| format!("{p:?}"))
            .filter(|p| !use_any_pseudo || !self.use_pseudos.contains(p))
            .last()
        else {
            log::debug!("selector doesn't end with a static token: {selector:?}");
            return true;
        };
        let token_type = match token.chars().next() {
            Some('.') => {
                token.remove(0);
                Token::Class
            }
            Some('#') => {
                token.remove(0);
                Token::ID
            }
            Some('[') => {
                token.pop();
                token.remove(0);
                Token::Attr
            }
            _ => Token::Other,
        };
        let id_name = &"id".into();
        let token = token.into();
        let match_count = matches
            .iter()
            .filter(|m| match token_type {
                Token::Class => Element::has_class(*m, &token),
                Token::ID => m
                    .get_attribute_local(id_name)
                    .is_some_and(|id| id.as_ref() == token.as_str()),
                Token::Attr => {
                    let Some((name, value)) = token.as_str().split_once('=') else {
                        return false;
                    };
                    let Some(attr) = m.get_attribute_local(&name.into()) else {
                        return false;
                    };
                    attr.as_str() == value
                }
                Token::Other => {
                    m.local_name().as_str() == token.as_str()
                        || !PRESERVED_PSEUDOS.contains(token.as_str())
                }
            })
            .count();
        log::debug!("selector {token:?} has matches: {match_count:?}");

        let removeable = if self.only_matched_once {
            match_count == 1
        } else {
            match_count > 0
        };
        if removeable {
            log::debug!("selector {token:?} removed");
            removed_selector.selectors.0.push(selector.clone());
        }
        removeable
    }

    fn strip_allowed_pseudos(&self, selector: String) -> String {
        let mut new_selector = None;
        for pseudo in &self.use_pseudos {
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
}

fn find_parent_attrs(style_rule: &rules::style::StyleRule) -> ParentTokens {
    macro_rules! get_selector_presentation_attrs {
        ($iter:ident) => {
            $iter
                .filter_map(|item| match item {
                    selector::Component::AttributeOther(a) => match a.namespace {
                        None => Some(&a.local_name_lower),
                        Some(_) => None,
                    },
                    selector::Component::AttributeInNoNamespace { local_name, .. } => {
                        Some(local_name)
                    }
                    selector::Component::AttributeInNoNamespaceExists {
                        local_name_lower, ..
                    } => Some(local_name_lower),
                    _ => None,
                })
                .filter_map(|s| s.to_css_string(printer::PrinterOptions::default()).ok())
                .filter(|s| PRESENTATION.contains(s.as_str()))
        };
    }

    ParentTokens {
        presentation_attrs: style_rule
            .selectors
            .0
            .iter()
            .flat_map(|s| {
                let iter = &mut s.iter();
                let mut attrs = vec![];
                attrs.extend(get_selector_presentation_attrs!(iter));
                while iter.next_sequence().is_some() {
                    attrs.extend(get_selector_presentation_attrs!(iter));
                }
                attrs
            })
            .collect(),
        classes: style_rule
            .selectors
            .0
            .iter()
            .flat_map(|s| {
                let iter = &mut s.iter();
                let mut classes = vec![];
                iter.for_each(|_| ());
                while iter.next_sequence().is_some() {
                    classes.extend(
                        iter.filter_map(|t| match t {
                            selector::Component::Class(class) => Some(class),
                            _ => None,
                        })
                        .map(|t| format!("{t}")),
                    );
                }
                classes
            })
            .collect(),
        ids: style_rule
            .selectors
            .0
            .iter()
            .flat_map(|s| {
                let iter = &mut s.iter();
                let mut ids = vec![];
                iter.for_each(|_| ());
                while iter.next_sequence().is_some() {
                    ids.extend(
                        iter.filter_map(|t| match t {
                            selector::Component::ID(id) => Some(id),
                            _ => None,
                        })
                        .map(|t| format!("{t}")),
                    );
                }
                ids
            })
            .collect(),
    }
}

impl<'arena, E: Element<'arena>> Clone for CapturedStyles<'arena, E> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            css: self.css.clone(),
            marker: PhantomData,
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

lazy_static! {
    static ref PRESERVED_PSEUDOS: BTreeSet<&'static str> = {
        PSEUDO_FUNCTIONAL
            .iter()
            .chain(PSEUDO_TREE_STRUCTURAL.iter())
            .map(AsRef::as_ref)
            .collect()
    };
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
    <!-- important styles take precendence over inline styles -->
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
        r#"{ "inlineStyles": { "useMqs": ["only screen and (device-width >= 320px) and (device-width <= 480px) and (-webkit-device-pixel-ratio >= 2)"] } }"#,
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
    <defs xmlns="http://www.w3.org/1999/xhtml">
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
            r#"<svg id="Ebene_1" data-name="Ebene 1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 222 57.28">
    <!-- ids and classes handled correctly -->
    <defs>
        <style>
            #id0 {
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
    <path class="cls-2" d="M312.75,168.66A2.15,2.15,0,0,1,311.2,165L316,160l-4.8-5a2.15,2.15,0,1,1,3.1-3l6.21,6.49a2.15,2.15,0,0,1,0,3L314.31,168a2.14,2.14,0,0,1-1.56.67Zm0,0" transform="translate(-119 -131.36)"/>
    <circle class="cls-2" cx="33.5" cy="27.25" r="2.94"/>
    <circle class="cls-2" cx="162.5" cy="158.61" r="2.94" transform="translate(-181.03 61.15) rotate(-52.89)"/>
    <circle class="cls-2" cx="172.5" cy="158.61" r="2.94" transform="translate(-157.03 -75.67) rotate(-16.55)"/>
</svg>"#
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

    Ok(())
}
