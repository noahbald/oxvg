use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use oxvg_ast::{
    attribute::{Attr, Attributes},
    element::Element,
    name::Name,
    node::{self, Node},
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::{
    collections::REFERENCES_PROPS,
    regex::{REFERENCES_BEGIN, REFERENCES_HREF, REFERENCES_URL},
};
use oxvg_derive::OptionalDefault;
use regex::CaptureMatches;
use serde::Deserialize;

use super::ContextFlags;

#[derive(Debug)]
struct ReplaceCounter(String, usize);

#[derive(Clone, Debug)]
struct GeneratedId {
    pub current: String,
    prevent_collision: BTreeSet<String>,
}

#[derive(Debug)]
struct RefRename {
    element_ref: Box<dyn node::Ref>,
    local_name: String,
    referenced_id: String,
}

#[derive(Debug, Deserialize, Default, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct CleanupIds {
    remove: Option<bool>,
    minify: Option<bool>,
    preserve: Option<Vec<String>>,
    preserve_prefixes: Option<Vec<String>>,
    force: Option<bool>,
    #[serde(skip_deserializing)]
    ignore_document: bool,
    #[serde(skip_deserializing)]
    replaceable_ids: BTreeSet<String>,
    #[serde(skip_deserializing)]
    id_renames: RefCell<BTreeMap<String, String>>,
    #[serde(skip_deserializing)]
    ref_renames: RefCell<Vec<RefRename>>,
    #[serde(skip_deserializing)]
    generated_id: RefCell<GeneratedId>,
}

impl<E: Element> Visitor<E> for CleanupIds {
    type Error = String;

    fn prepare(&mut self, document: &E, context_flags: &ContextFlags) -> PrepareOutcome {
        self.prepare_ignore_document(document, context_flags);
        if self.ignore_document {
            log::debug!("CleanupIds::prepare: skipping");
            return PrepareOutcome::skip;
        }

        self.prepare_id_rename(document);
        PrepareOutcome::none
    }

    fn element(&mut self, element: &mut E, _context: &Context<E>) -> Result<(), String> {
        if self.ignore_document {
            return Ok(());
        }

        let mut generated_id = self.generated_id.borrow_mut();
        // Find references in attributes
        let mut ref_renames = self.ref_renames.borrow_mut();
        for attr in element.attributes().iter() {
            let name = attr.name();
            let local_name = name.local_name();
            let value = attr.value();
            let Some(matches) = find_references(local_name.as_ref(), value.as_ref()) else {
                continue;
            };
            matches
                .filter_map(|item| item.get(1))
                .map(|item| item.as_str())
                .for_each(|item| {
                    if self.replaceable_ids.contains(item) {
                        log::debug!("CleanupIds::run: found potential reference: {item}");
                        ref_renames.push(RefRename {
                            element_ref: element.as_ref(),
                            local_name: name.to_string(),
                            referenced_id: item.to_string(),
                        });
                    } else {
                        log::debug!("CleanupIds::run: found unmatched reference: {item}");
                        generated_id.insert_prevent_collision(item.to_string());
                    }
                });
        }
        Ok(())
    }

    fn exit_document(&mut self, document: &mut E) -> Result<(), String> {
        let remove = self.remove.unwrap_or(REMOVE_DEFAULT);

        let Some(root) = &document.find_element() else {
            return Ok(());
        };
        // Generate renames for references
        let mut used_ids = BTreeMap::new();
        let mut generated_id = self.generated_id.borrow_mut();
        for RefRename {
            element_ref,
            local_name,
            referenced_id,
        } in self.ref_renames.borrow().iter()
        {
            let element = element_ref
                .inner_as_any()
                .downcast_ref::<E::ParentChild>()
                .expect("cleanup ids used with inconsistent types");
            let element = element.element().expect("memoised invalid node type");
            let Some(ref mut attr) = element.get_attribute(&local_name.as_str().into()) else {
                log::debug!("CleanupIds::breakdown: {local_name} attribute missing");
                continue;
            };
            let minified_id = used_ids
                .get(referenced_id)
                .unwrap_or(&generated_id.current)
                .clone();
            let replacements = replace_id_in_attr(attr.to_string(), referenced_id, &minified_id);
            if replacements.count() == &0 {
                continue;
            }
            let is_new = used_ids
                .insert(referenced_id.clone(), minified_id.clone())
                .is_none();
            if is_new {
                generated_id.next();
            }
            if self.minify.unwrap_or(MINIFY_DEFAULT) {
                log::debug!(
                    "CleanupIds::breakdown: updating reference: {local_name} <-> {referenced_id}"
                );
                element.set_attribute(local_name.as_str().into(), replacements.0.as_str().into());
            }
        }
        log::debug!(
            "CleanupIds::breakdown: replacing: {:#?} <-> {:#?}",
            &self.id_renames,
            &used_ids,
        );
        let id_localname = "id".into();
        for element in root.select("[id]").unwrap() {
            let Some(id) = element.get_attribute_local(&id_localname) else {
                continue;
            };
            let id = id.to_string();
            if let Some(rename) = used_ids
                .get(&id)
                .or_else(|| used_ids.get(&urlencoding::encode(&id).to_string()))
            {
                element.set_attribute_local(id_localname.clone(), rename.to_string().into());
            } else if remove && self.replaceable_ids.contains(&id) {
                element.remove_attribute_local(&id_localname);
            };
        }
        Ok(())
    }
}

impl CleanupIds {
    fn prepare_ignore_document(&mut self, root: &impl Element, context_flags: &ContextFlags) {
        if self.force == Some(true) {
            // Then we don't care, just pretend we don't have a script or style
            self.ignore_document = false;
            return;
        }

        let contains_unpredictable_refs = context_flags.contains(ContextFlags::has_stylesheet)
            || context_flags.contains(ContextFlags::has_script_ref);
        let contains_only_defs = root.select("svg > :not(defs)").unwrap().next().is_none();
        self.ignore_document = contains_unpredictable_refs || contains_only_defs;
    }

    /// Prepares tracking of ids for removal/renaming
    /// - Adds non-preserved ids to `self.replaceable_ids`
    /// - Removes any duplicate replaceable ids
    fn prepare_id_rename(&mut self, root: &impl Element) {
        let mut preserved_ids = Vec::new();
        log::debug!(
            "CleanupIds: prepare_id: preserve: {:#?} <-> {:#?}",
            &self.preserve,
            &self.preserve_prefixes
        );
        // Find ids
        let id_localname = &"id".into();
        for element in root.select("[id]").unwrap() {
            let Some(attr) = element.get_attribute_local(id_localname) else {
                continue;
            };
            let value = attr.to_string();
            log::debug!("CleanupIds: prepare_id: found id: {value}");
            if self.replaceable_ids.contains(&value) || value.chars().all(char::is_numeric) {
                element.remove_attribute_local(id_localname);
                log::debug!("CleanupIds: prepare_id: removed redundant id: {value}");
                continue;
            }
            let is_preserved_prefix = self
                .preserve_prefixes
                .as_ref()
                .is_some_and(|prefixes| prefixes.iter().any(|prefix| value.starts_with(prefix)));
            let is_preserve = self
                .preserve
                .as_ref()
                .is_some_and(|preserve| preserve.contains(&value));
            if is_preserved_prefix || is_preserve {
                preserved_ids.push(value);
                continue;
            }
            self.replaceable_ids.insert(value.clone());
            let encoded_id = urlencoding::encode(&value);
            if encoded_id != value {
                self.replaceable_ids.insert(encoded_id.to_string());
            }
        }
        self.generated_id
            .borrow_mut()
            .set_prevent_collision(preserved_ids);
    }
}

fn replace_id_in_attr(attr: String, id: &str, new_id: &str) -> ReplaceCounter {
    let has_hash = attr.contains('#');
    let mut replacer = ReplaceCounter::new(attr);
    if has_hash {
        replacer = replacer
            .replace(
                &format!("#{}", urlencoding::encode(id)),
                &format!("#{new_id}"),
            )
            .replace(&format!("#{id}"), &format!("#{new_id}"));
    } else {
        replacer = replacer.replace(&format!("{id}."), &format!("{new_id}."));
    }
    replacer
}

impl ReplaceCounter {
    fn new(value: String) -> ReplaceCounter {
        ReplaceCounter(value, 0)
    }

    /// An adaptation of `std::str::replace` with an additional counter
    fn replace(&mut self, from: &str, to: &str) -> ReplaceCounter {
        let string = &self.0;
        let mut result = String::new();
        let mut last_end = 0;
        for (start, part) in string.match_indices(from) {
            result.push_str(unsafe { string.get_unchecked(last_end..start) });
            result.push_str(to);
            last_end = start + part.len();
            self.1 += 1;
        }
        result.push_str(unsafe { string.get_unchecked(last_end..string.len()) });
        Self(result, self.1)
    }

    fn count(&self) -> &usize {
        &self.1
    }
}

impl From<&str> for ReplaceCounter {
    fn from(value: &str) -> Self {
        Self(value.to_string(), 0)
    }
}

impl GeneratedId {
    fn set_prevent_collision(&mut self, ids: Vec<String>) {
        self.prevent_collision = ids.into_iter().collect();
        if self.prevent_collision.contains(&self.current) {
            self.next();
        }
    }

    fn insert_prevent_collision(&mut self, id: String) {
        self.prevent_collision.insert(id);
        if self.prevent_collision.contains(&self.current) {
            self.next();
        }
    }
}

impl Iterator for GeneratedId {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut increment_next = true;
        let mut new_id: String = self
            .current
            .chars()
            .rev()
            .map(|char| {
                let mut char = char as u8;
                if increment_next {
                    char += 1;
                    increment_next = false;
                }
                if char > b'Z' && char < b'a' {
                    increment_next = true;
                    return 'a';
                } else if char > b'z' {
                    return 'A';
                }
                char::from(char)
            })
            .rev()
            .collect();
        if increment_next {
            new_id.insert(0, 'a');
        }
        self.current.clone_from(&new_id);
        if self.prevent_collision.contains(&new_id) {
            self.next()
        } else {
            Some(new_id)
        }
    }
}

impl Default for GeneratedId {
    fn default() -> Self {
        Self {
            current: String::from("a"),
            prevent_collision: BTreeSet::default(),
        }
    }
}

impl Clone for RefRename {
    fn clone(&self) -> Self {
        Self {
            element_ref: self.element_ref.clone(),
            local_name: self.local_name.clone(),
            referenced_id: self.referenced_id.clone(),
        }
    }
}

static REMOVE_DEFAULT: bool = true;
static MINIFY_DEFAULT: bool = true;

fn find_references<'a>(name: &str, value: &'a str) -> Option<CaptureMatches<'static, 'a>> {
    let matches = match name {
        "href" => REFERENCES_HREF.captures_iter(value),
        "begin" => REFERENCES_BEGIN.captures_iter(value),
        name if REFERENCES_PROPS.contains(name) => REFERENCES_URL.captures_iter(value),
        _ => return None,
    };
    Some(matches)
}

#[test]
#[allow(clippy::too_many_lines)]
fn cleanup_ids() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- Minify ids and references to ids -->
    <defs>
        <linearGradient id="gradient001">
            <stop offset="5%" stop-color="#F60"/>
            <stop offset="95%" stop-color="#FF6"/>
        </linearGradient>
        <text id="referencedText">
            referenced text
        </text>
        <path id="crochet" d="..."/>
        <path id="block" d="..."/>
        <path id="two" d="..."/>
        <path id="two" d="..."/>
    </defs>
    <g id="g001">
        <circle id="circle001" fill="url(#gradient001)" cx="60" cy="60" r="50"/>
        <rect fill="url('#gradient001')" x="0" y="0" width="500" height="100"/>
        <tref href="#referencedText"/>
    </g>
    <g>
        <tref href="#referencedText"/>
    </g>
    <animateMotion href="#crochet" dur="0.5s" begin="block.mouseover" fill="freeze" path="m 0,0 0,-21"/>
    <use href="#two"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- Ignore when <style> is present -->
    <style>
        .cls-1 { fill: #fff; }
    </style>
    <circle id="circle001" fill="red" cx="60" cy="60" r="50"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- Ignore when <script> is present -->
    <script>
        …
    </script>
    <circle id="circle001" fill="red" cx="60" cy="60" r="50"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:x="http://www.w3.org/1999/xlink">
    <!-- Minify ids and references to ids -->
    <defs>
        <g id="mid-line"/>
        <g id="line-plus">
            <use href="#mid-line"/>
            <use href="#plus"/>
        </g>
        <g id="plus"/>
        <g id="line-circle">
            <use href="#mid-line"/>
        </g>
    </defs>
    <path d="M0 0" id="a"/>
    <use href="#a" x="50" y="50"/>
    <use href="#line-plus"/>
    <use href="#line-circle"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "force": true
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- Allow minification when force is given, regardless of `<style>` -->
    <style>
        …
    </style>
    <circle id="circle001" fill="red" cx="60" cy="60" r="50"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "force": true
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- Allow minification when force is given, regardless of `<script>` -->
    <script>
        …
    </script>
    <circle id="circle001" fill="red" cx="60" cy="60" r="50"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "preserve": ["circle", "rect"]
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 230 120">
    <!-- Prevent modifications on preserved ids -->
    <circle id="circle001" fill="red" cx="60" cy="60" r="50"/>
    <rect id="rect001" fill="blue" x="120" y="10" width="100" height="100"/>
    <view id="circle" viewBox="0 0 120 120"/>
    <view id="rect" viewBox="110 0 120 120"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "force": true,
            "preserve": ["circle", "rect"]
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 120 120">
    <!-- Prevent modification on preserved ids, even in forced mode -->
    <style>
        svg .hidden { display: none; }
        svg .hidden:target { display: inline; }
    </style>
    <circle id="circle" class="hidden" fill="red" cx="60" cy="60" r="50"/>
    <rect id="rect" class="hidden" fill="blue" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "force": true,
            "preserve": ["figure"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 120 120">
    <!-- Prevent modification on preserved ids, even in forced mode -->
    <style>
        svg .hidden { display: none; }
        svg .hidden:target { display: inline; }
    </style>
    <defs>
        <circle id="circle" fill="red" cx="60" cy="60" r="50"/>
        <rect id="rect" fill="blue" x="10" y="10" width="100" height="100"/>
    </defs>
    <g id="figure" class="hidden">
        <use href="#circle"/>
        <use href="#rect"/>
    </g>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Ignore when svg's children are only <defs> -->
    <defs>
        <circle cx="100" cy="100" r="50" id="circle"/>
        <ellipse cx="50" cy="50" rx="50" ry="10" id="ellipse"/>
        <rect x="100" y="50" width="50" height="10" id="rect"/>
    </defs>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
        "preservePrefixes": ["xyz"]
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 230 120">
    <!-- Prevent modification of preserved id prefixes -->
    <circle id="garbage1" fill="red" cx="60" cy="60" r="50"/>
    <rect id="garbage2" fill="blue" x="120" y="10" width="100" height="100"/>
    <view id="xyzgarbage1" viewBox="0 0 120 120"/>
    <view id="xyzgarbage2" viewBox="110 0 120 120"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "force": true,
            "preservePrefixes": ["pre1_", "pre2_"]
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 120 120">
    <!-- Prevent modification of preserved id prefixes, even in forced mode -->
    <style>
        svg .hidden { display: none; }
        svg .hidden:target { display: inline; }
    </style>
    <circle id="pre1_circle" class="hidden" fill="red" cx="60" cy="60" r="50"/>
    <rect id="pre2_rect" class="hidden" fill="blue" x="10" y="10" width="100" height="100"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "force": true,
            "preserve": ["pre1_"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 120 120">
    <!-- Prevent modification of preserved id prefixes, even in forced mode -->
    <style>
        svg .hidden { display: none; }
        svg .hidden:target { display: inline; }
    </style>
    <defs>
        <circle id="circle" fill="red" cx="60" cy="60" r="50"/>
        <rect id="rect" fill="blue" x="10" y="10" width="100" height="100"/>
    </defs>
    <g id="pre1_figure" class="hidden">
        <use xlink:href="#circle"/>
        <use href="#rect"/>
    </g>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "preserve": ["circle"],
            "preservePrefixes": ["suffix", "rect"]
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 230 120">
    <!-- Preserve both preserved names and prefixes -->
    <circle id="circle" fill="red" cx="60" cy="60" r="50"/>
    <rect id="rect" fill="blue" x="120" y="10" width="100" height="100"/>
    <view id="circle-suffix" viewBox="0 0 120 120"/>
    <view id="rect-suffix" viewBox="110 0 120 120"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "preserve": ["a"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 230 120">
    <!-- Don't collide minification with preserved ids -->
    <defs>
        <circle id="a" fill="red" cx="60" cy="60" r="50"/>
        <rect id="rect" fill="blue" x="120" y="10" width="100" height="100"/>
    </defs>
    <use xlink:href="#a"/>
    <use href="#rect"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "preservePrefixes": ["a"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 230 120">
    <!-- Don't collide minification with preserved prefixes -->
    <defs>
        <circle id="a" fill="red" cx="60" cy="60" r="50"/>
        <rect id="rect" fill="blue" x="120" y="10" width="100" height="100"/>
    </defs>
    <use href="#a"/>
    <use href="#rect"/>
</svg>"##
        )
    )?);

    // WARN: This output is different to SVGO
    // SVGO: <use href="#rect"/> --> <use href="#b" />
    // OXVG: <use href="#rect"/> --> <use href="#a" />
    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "preservePrefixes": ["a"]
        } }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 230 120">
    <!-- Don't collide minification with preserved prefixes -->
    <defs>
        <circle id="abc" fill="red" cx="60" cy="60" r="50"/>
        <rect id="rect" fill="blue" x="120" y="10" width="100" height="100"/>
    </defs>
    <use href="#abc"/>
    <use href="#rect"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 48 48">
    <!-- Allow minification when <style> is empty -->
    <defs>
        <style></style>
        <linearGradient id="file-name_svg__file-name_svg__original-id" x1="12" y1="-1" x2="33" y2="46" gradientUnits="userSpaceOnUse">
            <stop offset="0" stop-color="#6b5aed" stop-opacity="0" />
            <stop offset="1" stop-color="#6b5aed" />
        </linearGradient>
    </defs>
    <path d="M46 24a21.9 21.9" fill="url(#file-name_svg__file-name_svg__original-id)"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {
            "remove": false
        } }"#,
        Some(
            r##"<svg width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
    <!-- Prevent removal of ids -->
    <g filter="url(#filter0_dust)">
        <path d="M2 8a7 7 0 1 1 14 0A7 7 0 0 1 2 8z" fill="#fff"/>
    </g>
    <path d="M4 8a5 5 0 1 1 10 0A5 5 0 0 1 4 8z" fill="currentColor"/>
    <defs>
        <filter id="filter0_dust" x="0" y="0" width="18" height="18" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
            <feFlood flood-opacity="0" result="BackgroundImageFix"/>
            <feColorMatrix in="SourceAlpha" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"/>
            <feOffset dy="1"/>
            <feGaussianBlur stdDeviation="1"/>
            <feColorMatrix values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.2 0"/>
            <feBlend in2="BackgroundImageFix" result="effect1_dropShadow"/>
            <feBlend in="SourceGraphic" in2="effect1_dropShadow" result="shape"/>
        </filter>
    </defs>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg width="379px" height="134px" viewBox="0 0 379 134" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <!-- Remove unreferenced ids -->
    <circle id="6" cx="110.5" cy="5.5" r="5.5">
        <animate begin="2.5s" attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
    </circle>
    <circle id="5" cx="89.5" cy="5.5" r="5.5">
        <animate begin="2s" attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
    </circle>
    <circle id="4" cx="68.5" cy="5.5" r="5.5">
        <animate begin="1.5s" attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
    </circle>
    <circle id="3" cx="47.5" cy="5.5" r="5.5">
        <animate begin="1s" attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
    </circle>
    <circle id="2" cx="26.5" cy="5.5" r="5.5">
        <animate begin="0.5s" attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
    </circle>
    <circle id="1" cx="5.5" cy="5.5" r="5.5">
        <animate attributeName="fill" calcMode="discrete" values="#6ebe28;#D8D8D8" dur="5s" keyTimes="0;0.15" repeatCount="indefinite"/>
    </circle>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="1950.1315" height="1740.1298">
  <!-- Unchanged ids are still referenced correctly -->
  <linearGradient id="a">
    <stop stop-color="#f00" offset="0"/>
  </linearGradient>
  <linearGradient id="linearGradient3520" href="#a" gradientUnits="userSpaceOnUse" gradientTransform="translate(7991.4092,-7484.0182)" x1="475.01208" y1="29234.521" x2="-1343.6307" y2="29445.83"/>
  <filter id="c" style="color-interpolation-filters:sRGB" x="-0.2760295" width="1.5520591" y="-0.33142158" height="1.6628431">
    <feGaussianBlur stdDeviation="331.22039"/>
  </filter>
  <g transform="matrix(5.8862959,0,0,5.8862959,-228.3949,1414.6785)">
    <path d="m 6416.0915,21026.021 c 496.2734,-430.162 1156.7926,-524.889 1495.2326,-581.643 1461.5227,-245.087 1539.467,2033.775 96.1224,2234.099 -524.6707,72.82 -1265.3758,450.675 -1679.5812,-402.754 -315.0174,-535.208 -91.5956,-1058.609 88.2262,-1249.702 z" style="opacity:1;fill:url(#linearGradient3520);fill-opacity:1;stroke:none;stroke-width:16.60000038;stroke-linecap:butt;stroke-linejoin:miter;stroke-miterlimit:4;stroke-dasharray:none;stroke-opacity:1;filter:url(#c)" transform="matrix(0.07412091,0,0,0.07412091,-359.59058,-1695.4044)"/>
  </g>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
  <!--
  When a reference to a non-existant id would be created by minification, try the next
  possible generated id
  -->
  <defs>
    <path id="uwu" d="M 2.046875 0 L 10.609375 0 C 12.40625 0 13.734375 -0.5 14.734375 -1.59375 C 15.671875 -2.578125 16.203125 -3.921875 16.203125 -5.40625 C 16.203125 -7.703125 15.15625 -9.078125 12.734375 -10.015625 C 14.484375 -10.8125 15.359375 -12.1875 15.359375 -14.140625 C 15.359375 -15.546875 14.84375 -16.75 13.859375 -17.625 C 12.84375 -18.53125 11.5625 -18.953125 9.75 -18.953125 L 2.046875 -18.953125 Z M 4.46875 -10.796875 L 4.46875 -16.828125 L 9.15625 -16.828125 C 10.5 -16.828125 11.265625 -16.640625 11.90625 -16.140625 C 12.578125 -15.625 12.953125 -14.84375 12.953125 -13.8125 C 12.953125 -12.765625 12.578125 -11.984375 11.90625 -11.46875 C 11.265625 -10.96875 10.5 -10.796875 9.15625 -10.796875 Z M 4.46875 -2.125 L 4.46875 -8.65625 L 10.375 -8.65625 C 12.5 -8.65625 13.78125 -7.4375 13.78125 -5.375 C 13.78125 -3.359375 12.5 -2.125 10.375 -2.125 Z M 4.46875 -2.125"/>
  </defs>
  <use href="#a" x="378" y="464"/>
  <use href="#uwu" x="385" y="464"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <!-- Rename within animation references, eg "<id>.<property>" -->
  <circle cx="12" cy="12">
    <animate id="thing1" fill="freeze" attributeName="r" begin="0;thing2.end" dur="1.2s" values="0;11"/>
  </circle>
  <circle cx="12" cy="12">
    <animate id="thing2" fill="freeze" attributeName="r" begin="thing1.begin+0.2s" dur="1.2s" values="0;11"/>
  </circle>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 9 9">
  <!-- Handle non-ascii and URI encoding correctly -->
  <defs>
    <path id="人口" d="M1 1l2 2" stroke="black"/>
  </defs>
  <use href="#%E4%BA%BA%E5%8F%A3"/>
</svg>"##
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Handle non-ascii and URI encoding correctly -->
    <defs>
        <linearGradient id="渐变_1" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop stop-color="#5a2100" />
        </linearGradient>
    </defs>
    <rect x="30" y="30" height="150" width="370" fill="url(#渐变_1)" />
</svg>"##
        )
    )?);

    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn cleanup_ids_check_rename() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        // Minifies ids should sequences from "a..z", "A..Z", "aa..az", and so on
        r#"{ "cleanupIds": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <defs>
        <text id="__proto__">
            referenced text
        </text>
        <text id="test02">
            referenced text
        </text>
        <text id="test03">
            referenced text
        </text>
        <text id="test04">
            referenced text
        </text>
        <text id="test05">
            referenced text
        </text>
        <text id="test06">
            referenced text
        </text>
        <text id="test07">
            referenced text
        </text>
        <text id="test08">
            referenced text
        </text>
        <text id="test09">
            referenced text
        </text>
        <text id="test10">
            referenced text
        </text>
        <text id="test11">
            referenced text
        </text>
        <text id="test12">
            referenced text
        </text>
        <text id="test13">
            referenced text
        </text>
        <text id="test14">
            referenced text
        </text>
        <text id="test15">
            referenced text
        </text>
        <text id="test16">
            referenced text
        </text>
        <text id="test17">
            referenced text
        </text>
        <text id="test18">
            referenced text
        </text>
        <text id="test19">
            referenced text
        </text>
        <text id="test20">
            referenced text
        </text>
        <text id="test21">
            referenced text
        </text>
        <text id="test22">
            referenced text
        </text>
        <text id="test23">
            referenced text
        </text>
        <text id="test24">
            referenced text
        </text>
        <text id="test25">
            referenced text
        </text>
        <text id="test26">
            referenced text
        </text>
        <text id="test27">
            referenced text
        </text>
        <text id="test28">
            referenced text
        </text>
        <text id="test29">
            referenced text
        </text>
        <text id="test30">
            referenced text
        </text>
        <text id="test31">
            referenced text
        </text>
        <text id="test32">
            referenced text
        </text>
        <text id="test33">
            referenced text
        </text>
        <text id="test34">
            referenced text
        </text>
        <text id="test35">
            referenced text
        </text>
        <text id="test36">
            referenced text
        </text>
        <text id="test37">
            referenced text
        </text>
        <text id="test38">
            referenced text
        </text>
        <text id="test39">
            referenced text
        </text>
        <text id="test40">
            referenced text
        </text>
        <text id="test41">
            referenced text
        </text>
        <text id="test42">
            referenced text
        </text>
        <text id="test43">
            referenced text
        </text>
        <text id="test44">
            referenced text
        </text>
        <text id="test45">
            referenced text
        </text>
        <text id="test46">
            referenced text
        </text>
        <text id="test47">
            referenced text
        </text>
        <text id="test48">
            referenced text
        </text>
        <text id="test49">
            referenced text
        </text>
        <text id="test50">
            referenced text
        </text>
        <text id="test51">
            referenced text
        </text>
        <text id="test52">
            referenced text
        </text>
        <text id="test53">
            referenced text
        </text>
    </defs>
    <tref href="#__proto__"/>
    <tref href="#__proto__"/>
    <tref href="#__proto__"/>
    <tref href="#test02"/>
    <tref href="#test03"/>
    <tref href="#test04"/>
    <tref href="#test05"/>
    <tref href="#test06"/>
    <tref href="#test07"/>
    <tref href="#test08"/>
    <tref href="#test09"/>
    <tref href="#test10"/>
    <tref href="#test11"/>
    <tref href="#test12"/>
    <tref href="#test13"/>
    <tref href="#test14"/>
    <tref href="#test15"/>
    <tref href="#test16"/>
    <tref href="#test17"/>
    <tref href="#test18"/>
    <tref href="#test19"/>
    <tref href="#test20"/>
    <tref href="#test21"/>
    <tref href="#test22"/>
    <tref href="#test23"/>
    <tref href="#test24"/>
    <tref href="#test25"/>
    <tref href="#test26"/>
    <tref href="#test27"/>
    <tref href="#test28"/>
    <tref href="#test29"/>
    <tref href="#test30"/>
    <tref href="#test31"/>
    <tref href="#test32"/>
    <tref href="#test33"/>
    <tref href="#test34"/>
    <tref href="#test35"/>
    <tref href="#test36"/>
    <tref href="#test37"/>
    <tref href="#test38"/>
    <tref href="#test39"/>
    <tref href="#test40"/>
    <tref href="#test41"/>
    <tref href="#test42"/>
    <tref href="#test43"/>
    <tref href="#test44"/>
    <tref href="#test45"/>
    <tref href="#test46"/>
    <tref href="#test47"/>
    <tref href="#test48"/>
    <tref href="#test49"/>
    <tref href="#test50"/>
    <tref href="#test51"/>
    <tref href="#test52"/>
    <tref href="#test53"/>
</svg>"##
        )
    )?);

    Ok(())
}
