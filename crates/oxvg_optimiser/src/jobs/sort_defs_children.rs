use std::{cmp::Ordering, collections::HashMap};

use oxvg_ast::{
    element::Element,
    is_element,
    visitor::{Context, PrepareOutcome, Visitor},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
/// Sorts the children of `<defs>` into a predictable order.
///
/// This doesn't affect the size of a document but will likely improve readability
/// and compression of the document.
///
/// # Correctness
///
/// This job may affect the document if selectors or scripts depend on ordering.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct SortDefsChildren(pub bool);

impl<'input, 'arena> Visitor<'input, 'arena> for SortDefsChildren {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        _document: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        Ok(if self.0 {
            PrepareOutcome::none
        } else {
            PrepareOutcome::skip
        })
    }

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        _context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !is_element!(element, Defs) {
            return Ok(());
        }

        let mut frequencies = HashMap::new();
        element.children_iter().for_each(|e| {
            let name = e.qual_name();
            if let Some(frequency) = frequencies.get_mut(name) {
                *frequency += 1;
            } else {
                frequencies.insert(name.clone(), 1);
            }
        });
        element.sort_child_elements(|a, b| {
            let a_name = a.qual_name();
            let b_name = b.qual_name();
            let a_frequency = frequencies.get(a_name);
            let b_frequency = frequencies.get(b_name);
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
            b_name.cmp(a_name)
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
