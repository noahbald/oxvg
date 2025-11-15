use std::collections::HashSet;

use oxvg_ast::{
    element::Element,
    get_attribute_mut, is_element,
    visitor::{Context, Visitor},
};
use oxvg_collections::{
    atom::Atom,
    attribute::{
        core::NonWhitespace,
        list_of::{ListOf, Space},
    },
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
/// Adds to the `class` attribute of the root `<svg>` element, omitting duplicates
///
/// # Differences to SVGO
///
/// The order of CSS classes may not be applied in the order given.
///
/// # Examples
///
/// Use with a list of classes
///
/// ```ignore
/// use oxvg_optimiser::{Jobs, AddClassesToSVG};
///
/// let jobs = Jobs {
///   add_classes_to_svg: Some(AddClassesToSVG {
///     class_names: Some(vec![String::from("foo"), String::from("bar")]),
///     ..AddClassesToSVG::default()
///   }),
///   ..Jobs::none()
/// };
/// ```
///
/// Use with a class string
///
/// ```ignore
/// use oxvg_optimiser::{Jobs, AddClassesToSVG};
///
/// let jobs = Jobs {
///   add_classes_to_svg: Some(AddClassesToSVG {
///     class_name: Some(String::from("foo bar")),
///     ..AddClassesToSVG::default()
///   }),
///   ..Jobs::none()
/// };
/// ```
///
///
/// # Correctness
///
/// This job may visually change documents if an added classname causes it to be
/// selected by CSS.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct AddClassesToSVGElement {
    /// Adds each class to the `class` attribute.
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub class_names: Option<Vec<String>>,
    /// Adds the classes to the `class` attribute, removing any whitespace between each. This option
    /// is ignored if `class_names` is provided.
    #[cfg_attr(feature = "wasm", tsify(optional))]
    pub class_name: Option<String>,
}

impl<'input, 'arena> Visitor<'input, 'arena> for AddClassesToSVGElement {
    type Error = JobsError<'input>;

    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        if !element.is_root() || !is_element!(element, Svg) {
            return Ok(());
        }

        let Some(mut class) = get_attribute_mut!(element, Class) else {
            return Ok(());
        };

        match &self.class_names {
            Some(names) => {
                let mut set = HashSet::new();
                for item in class.list.drain(..) {
                    set.insert(item);
                }
                set.extend(
                    names
                        .iter()
                        .map(|name| &*context.info.allocator.alloc_str(name))
                        .map(Into::into),
                );
                class.list = set.into_iter().collect();
            }
            None => match &self.class_name {
                Some(name) => {
                    *class = ListOf {
                        list: name
                            .split_whitespace()
                            .map(Atom::from)
                            .map(Atom::into_owned)
                            .map(NonWhitespace)
                            .collect(),
                        seperator: Space,
                    }
                }
                None => return Ok(()),
            },
        }
        Ok(())
    }
}

#[test]
fn add_classes_to_svg() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(crate::test_config!(
        r#"{ "addClassesToSvg": {
            "classNames": ["mySvg", "size-big"]
        } }"#,
        comment: "Should add classes when passed as a classNames Array"
    )?);

    insta::assert_snapshot!(crate::test_config!(
        r#"{ "addClassesToSvg": {
            "className": "mySvg"
        } }"#,
        comment: "Should add class when passed as a className String"
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "addClassesToSvg": {
            "className": "mySvg size-big"
        } }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" class="mySvg">
    <!-- Should avoid adding existing classes -->
    test
</svg>"#
        )
    )?);
    Ok(())
}
