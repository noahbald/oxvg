use lightningcss::values::shape::FillRule;
use oxvg_ast::{
    element::Element,
    get_attribute, get_computed_style, has_attribute, is_element, set_attribute,
    style::{self, ComputedStyles},
};
use oxvg_collections::attribute::{core_attrs::Integer, inheritable::Inheritable};
use oxvg_path::{algorithm::bool_ops::OverlayRule, geometry::Tolerance, paths::segment};

use crate::{Action, Actor, Error, utils::to_id};

impl<'input> Actor<'input, '_> {
    /// Intersects selected path definitions.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/manipulate/path_intersect.md")]
    pub fn path_intersect(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::PathIntersect);
        if let Some(selection) = self.boolean_op(OverlayRule::Intersect)? {
            self.effect_selection(&selection.into())?;
        }
        self.effect_tree()?;
        self.effect_document()
    }

    /// Unites selected path definitions.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/manipulate/path_union.md")]
    pub fn path_union(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::PathUnion);
        if let Some(selection) = self.boolean_op(OverlayRule::Union)? {
            self.effect_selection(&selection.into())?;
        }
        self.effect_tree()?;
        self.effect_document()
    }

    /// Subtracts selected path definitions.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/manipulate/path_subtract.md")]
    pub fn path_subtract(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::PathSubtract);
        if let Some(selection) = self.boolean_op(OverlayRule::Difference)? {
            self.effect_selection(&selection.into())?;
        }
        self.effect_tree()?;
        self.effect_document()
    }

    /// XORs selected path definitions.
    ///
    /// # Errors
    ///
    /// When root element is missing.
    ///
    /// # Spec
    ///
    #[doc = include_str!("../../spec/manipulate/path_xor.md")]
    pub fn path_xor(&mut self) -> Result<(), Error<'input>> {
        self.effect_history(&Action::PathXor);
        if let Some(selection) = self.boolean_op(OverlayRule::Xor)? {
            self.effect_selection(&selection.into())?;
        }
        self.effect_tree()?;
        self.effect_document()
    }

    fn boolean_op(&mut self, overlay_rule: OverlayRule) -> Result<Option<Integer>, Error<'input>> {
        let Some(selections) = self.get_selections()? else {
            return Ok(None);
        };
        let Some(root) = Element::from_parent(self.root) else {
            return Ok(None);
        };
        let mut cumulative_path: Option<oxvg_path::paths::bool::Path> = None;
        let mut previous_element: Option<Element> = None;
        let styles: Vec<_> = style::root(root).collect();
        let paths: Vec<_> = self
            .get_selection_elements(Some(selections))
            .filter(|e| !e.has_child_nodes() && is_element!(e, Path) && has_attribute!(e, D))
            .collect();
        for path in paths {
            let d = get_attribute!(path, D).unwrap();
            let segment_path = segment::Path::from_svg(&d, &Tolerance::default());
            drop(d);
            let computed_styles = ComputedStyles::default()
                .with_all(path, &styles)
                .map_err(|err| Error::ComputedStylesError(err.to_string()))?;
            let evenodd = get_computed_style!(computed_styles, FillRule)
                .option()
                .is_some_and(|fill_rule| match fill_rule {
                    (Inheritable::Defined(FillRule::Nonzero) | Inheritable::Inherited, _) => false,
                    (Inheritable::Defined(FillRule::Evenodd), _) => true,
                });
            let segment_path = oxvg_path::paths::bool::Path {
                inner: segment_path,
                evenodd,
            };

            cumulative_path = Some(match cumulative_path {
                Some(inner) => oxvg_path::paths::bool::Path {
                    inner: inner.boolean_op(&segment_path, overlay_rule),
                    evenodd: true,
                },
                None => segment_path,
            });

            if let Some(previous_element) = previous_element {
                previous_element.remove();
            }
            previous_element = Some(path);
        }

        if let (Some(final_element), Some(cumulative_path)) = (previous_element, cumulative_path) {
            set_attribute!(
                final_element,
                D(oxvg_collections::attribute::path::Path(
                    cumulative_path.inner.to_svg(&Tolerance::default(), true),
                    None
                ))
            );
            set_attribute!(
                final_element,
                FillRule(Inheritable::Defined(FillRule::Evenodd))
            );
        }
        Ok(previous_element.map(|e| to_id(*e)))
    }
}

#[cfg(test)]
mod test {
    use oxvg_ast::serialize::Node as _;

    use crate::Actor;

    #[test]
    fn intersect() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 2 2h8v8H2z" />
    <path d="M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z" fill-rule="evenodd" />
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                actor.path_intersect().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn union() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 2 2h8v8H2z" />
    <path d="M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z" fill-rule="evenodd" />
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                actor.path_union().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn subtract() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 2 2h8v8H2z" />
    <path d="M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z" fill-rule="evenodd" />
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                actor.path_subtract().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }

    #[test]
    fn xor() {
        oxvg_ast::parse::roxmltree::parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 2 2h8v8H2z" />
    <path d="M7 7a4 4 0 1 0 0.001 -0.001zM7.35 7.35 a3.5 3.5 0 1 0 0.001 -0.001z" fill-rule="evenodd" />
</svg>"#,
            |root, allocator| {
                let mut actor = Actor::new(root, allocator).unwrap();

                actor.select("path").unwrap();
                actor.path_xor().unwrap();
                insta::assert_snapshot!(actor.root.serialize().unwrap());
                insta::assert_debug_snapshot!(actor.derive_state().unwrap());
            },
        )
        .unwrap();
    }
}
