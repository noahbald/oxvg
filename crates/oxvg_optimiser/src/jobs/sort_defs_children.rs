use std::{cmp::Ordering, collections::HashMap};

use oxvg_ast::{
    element::Element,
    name::Name,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct SortDefsChildren(bool);

impl<E: Element> Visitor<E> for SortDefsChildren {
    type Error = String;

    fn prepare(
        &mut self,
        _document: &E,
        _context_flags: &mut oxvg_ast::visitor::ContextFlags,
    ) -> oxvg_ast::visitor::PrepareOutcome {
        if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        }
    }

    fn element(&mut self, element: &mut E, _context: &mut Context<E>) -> Result<(), String> {
        if element.prefix().is_some() || element.local_name().as_ref() != "defs" {
            return Ok(());
        }

        let mut frequencies = HashMap::new();
        element.for_each_element_child(|e| {
            let name = e.qual_name();
            if let Some(frequency) = frequencies.get_mut(&name) {
                *frequency += 1;
            } else {
                frequencies.insert(name, 1);
            }
        });
        element.sort_child_elements(|a, b| {
            let a_name = a.qual_name();
            let b_name = b.qual_name();
            let a_frequency = frequencies.get(&a_name);
            let b_frequency = frequencies.get(&b_name);
            if let Some(a_frequency) = a_frequency {
                if let Some(b_frequency) = b_frequency {
                    let frequency_ord = b_frequency.cmp(a_frequency);
                    if frequency_ord != Ordering::Equal {
                        return frequency_ord;
                    }
                }
            }
            let len_ord = b_name.len().cmp(&a_name.len());
            if len_ord != Ordering::Equal {
                return len_ord;
            }
            b_name.cmp(&a_name)
        });

        Ok(())
    }
}

impl Default for SortDefsChildren {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn sort_defs_children() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "sortDefsChildren": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <defs>
        <text id="a">
            referenced text
        </text>
        <path id="b" d="M0 0zM10 10zM20 20l10 10M30 0c10 0 20 10 20 20M30 30z"/>
        <text id="c">
            referenced text
        </text>
        <path id="d" d="M 30,30 z"/>
        <circle id="e" fill="none" fill-rule="evenodd" cx="60" cy="60" r="50"/>
        <circle id="f" fill="none" fill-rule="evenodd" cx="60" cy="60" r="50"/>
    </defs>
</svg>"#
        ),
    )?);

    Ok(())
}
