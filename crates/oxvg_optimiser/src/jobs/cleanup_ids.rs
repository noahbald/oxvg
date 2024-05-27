use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use markup5ever::{local_name, tendril::StrTendril};
use oxvg_selectors::Element;
use serde::Deserialize;

use crate::{Job, PrepareOutcome};

enum ReplaceCounter<'a> {
    String(String, usize),
    Tentril(&'a StrTendril, usize),
}

#[derive(Deserialize, Default, Clone)]
pub struct CleanupIds {
    remove: Option<bool>,
    minify: Option<bool>,
    preserve: Option<Vec<String>>,
    preserve_prefixes: Option<Vec<String>>,
    force: Option<bool>,
    #[serde(skip_deserializing)]
    ignore_document: bool,
    #[serde(skip_deserializing)]
    id_rename: HashMap<String, String>,
    #[serde(skip_deserializing)]
    unused_ids: RefCell<HashSet<String>>,
}

impl Job for CleanupIds {
    fn prepare(&mut self, document: &rcdom::RcDom) -> PrepareOutcome {
        let Some(root) = &Element::from_document_root(document) else {
            return PrepareOutcome::None;
        };
        self.prepare_ignore_document(root);
        if self.ignore_document {
            return PrepareOutcome::Skip;
        }

        self.prepare_id_rename(root);
        PrepareOutcome::None
    }

    fn run(&self, node: &Rc<rcdom::Node>) {
        use rcdom::NodeData::Element as ElementData;

        if self.ignore_document {
            return;
        }

        let ElementData { attrs, .. } = &node.data else {
            return;
        };

        let attrs = &mut *attrs.borrow_mut();
        for (id, new_id) in &self.id_rename {
            for attr in attrs.iter_mut() {
                let mut replacer = ReplaceCounter::from(&attr.value);
                if attr.value.contains('#') {
                    replacer = replacer
                        .replace(
                            &format!("#{}", urlencoding::encode(id)),
                            &format!("#{new_id}"),
                        )
                        .replace(&format!("#{id}"), &format!("#{new_id}"));
                } else {
                    replacer = replacer.replace(&format!("{id}."), &format!("{new_id}."));
                }
                if replacer.count() > &0 {
                    self.unused_ids.borrow_mut().remove(id);
                }
                if self.minify.is_some_and(|minify| minify) {
                    attr.value = replacer.into();
                }
            }
        }
    }

    fn breakdown(&mut self, document: &rcdom::RcDom) {
        use rcdom::NodeData::Element as ElementData;

        if !self.remove.is_some_and(|remove| remove) {
            return;
        }

        let Some(root) = &Element::from_document_root(document) else {
            return;
        };
        for id in &*self.unused_ids.borrow() {
            let Ok(mut element) = root.select(&format!("#{id}")) else {
                continue;
            };
            let Some(element) = element.next() else {
                continue;
            };
            let element = &element.node.data;
            let ElementData { attrs, .. } = element else {
                continue;
            };
            attrs
                .borrow_mut()
                .retain_mut(|attr| attr.name.local != local_name!("id"));
        }
    }
}

impl CleanupIds {
    fn prepare_ignore_document(&mut self, root: &Element) {
        if self.force == Some(true) {
            // Then we don't care, just pretend we don't have a script or style
            self.ignore_document = false;
            return;
        }

        let contains_unpredictable_refs = root.select("script, style").unwrap().next().is_some();
        let contains_only_defs = root.select(":scope > :not(defs)").unwrap().next().is_none();
        self.ignore_document = contains_unpredictable_refs || contains_only_defs;
    }

    fn prepare_id_rename(&mut self, root: &Element) {
        let elements_with_id = root.select("[id]").unwrap();
        let mut current_id_gen = String::from("a");
        for element in elements_with_id {
            let Some(attr) = element.get_attr(&local_name!("id")) else {
                continue;
            };
            if self.id_rename.contains_key(&attr.value.to_string()) {
                continue;
            }

            let is_preserved_prefix = self.preserve_prefixes.as_ref().is_some_and(|prefixes| {
                prefixes.iter().any(|prefix| attr.value.starts_with(prefix))
            });
            let is_preserve = self
                .preserve
                .as_ref()
                .is_some_and(|preserve| preserve.contains(&attr.value.clone().into()));
            if is_preserved_prefix || is_preserve {
                continue;
            }

            current_id_gen = Self::generate_id(&current_id_gen);
            self.id_rename
                .insert(attr.value.to_string(), current_id_gen.clone());
        }
        self.unused_ids = RefCell::new(self.id_rename.keys().cloned().collect());
    }

    fn generate_id(current_id: &str) -> String {
        let mut increment_next = true;
        let mut new_id: String = current_id
            .chars()
            .rev()
            .map(|char| {
                let mut char = char as u8;
                if increment_next {
                    char += 1;
                    increment_next = false;
                }
                if char > b'Z' {
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
        new_id
    }
}

impl<'a> ReplaceCounter<'a> {
    /// An adaptation of `std::str::replace` with an additional coutner
    fn replace(&self, from: &str, to: &str) -> Self {
        let mut unwrapped: (String, usize) = match self {
            Self::String(str, count) => (str.to_string(), *count),
            Self::Tentril(tendril, count) => (tendril.to_string(), *count),
        };
        let string = &unwrapped.0;
        let mut result = String::new();
        let mut last_end = 0;
        for (start, part) in string.match_indices(from) {
            result.push_str(unsafe { string.get_unchecked(last_end..start) });
            result.push_str(to);
            last_end = start + part.len();
            unwrapped.1 += 1;
        }
        result.push_str(unsafe { string.get_unchecked(last_end..string.len()) });
        Self::String(result, unwrapped.1)
    }

    fn count(&self) -> &usize {
        match self {
            Self::Tentril(_, count) | Self::String(_, count) => count,
        }
    }
}

impl<'a> From<&'a StrTendril> for ReplaceCounter<'a> {
    fn from(value: &'a StrTendril) -> Self {
        Self::Tentril(value, 0)
    }
}

impl From<ReplaceCounter<'_>> for StrTendril {
    fn from(value: ReplaceCounter<'_>) -> Self {
        match value {
            ReplaceCounter::String(string, _) => string.into(),
            ReplaceCounter::Tentril(tendril, _) => tendril.clone(),
        }
    }
}

#[test]
fn cleanup_enable_background() -> Result<(), serde_json::Error> {
    use xml5ever::{
        driver::{parse_document, XmlParseOpts},
        tendril::TendrilSink,
    };

    let dom: rcdom::RcDom = parse_document(rcdom::RcDom::default(), XmlParseOpts::default())
        .one(r#"<svg width=".5" height="10" enable-background="new 0 0 .5 10"></svg>"#);
    let job: crate::Jobs = serde_json::from_str(
        r#"{ "cleanup_ids": {
            "remove": true,
            "minify": true
        } }"#,
    )?;
    job.run(&dom);
    insta::assert_debug_snapshot!(&dom.document);
    Ok(())
}
