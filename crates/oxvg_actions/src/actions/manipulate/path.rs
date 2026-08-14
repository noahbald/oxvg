use lightningcss::values::shape::FillRule;
use oxvg_ast::{
    element::Element,
    get_attribute, get_computed_style, has_attribute, is_element, set_attribute,
    style::{self, ComputedStyles},
};
use oxvg_collections::attribute::inheritable::Inheritable;
use oxvg_path::{algorithm::bool_ops::OverlayRule, geometry::Tolerance, paths::segment};

use crate::{Action, Actor, Error, state::StateElement, utils::create_oxvg_attr};

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
        self.boolean_op(&Action::PathIntersect, OverlayRule::Intersect)
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
        self.boolean_op(&Action::PathUnion, OverlayRule::Union)
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
        self.boolean_op(&Action::PathSubtract, OverlayRule::Difference)
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
        self.boolean_op(&Action::PathXor, OverlayRule::Xor)
    }

    fn boolean_op(
        &mut self,
        action: &Action<'input>,
        overlay_rule: OverlayRule,
    ) -> Result<(), Error<'input>> {
        self.state.record(action, &self.allocator);
        let Some(selections) = self.get_selections()? else {
            return Ok(());
        };
        let Some(root) = Element::from_parent(self.root) else {
            return Ok(());
        };
        let mut cumulative_path: Option<oxvg_path::paths::bool::Path> = None;
        let mut previous_element: Option<Element> = None;
        let styles: Vec<_> = style::root(&root).collect();
        #[allow(clippy::cast_sign_loss)]
        let paths: Vec<_> = selections
            .iter()
            .filter_map(|s| self.allocator.get(*s as usize))
            .filter_map(oxvg_ast::node::Node::element)
            .filter(|e| !e.has_child_nodes() && is_element!(e, Path) && has_attribute!(e, D))
            .collect();
        for path in paths {
            let d = get_attribute!(path, D).unwrap();
            let segment_path = segment::Path::from_svg(&d, &Tolerance::default());
            drop(d);
            let computed_styles = ComputedStyles::default()
                .with_all(&path, &styles)
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

        if let Some(selection) = selections.last() {
            self.state
                .get_selections(&self.allocator)
                .set_attribute(create_oxvg_attr(
                    StateElement::SELECTION_IDS,
                    #[allow(clippy::cast_sign_loss)]
                    ((*selection as usize) - selections.len())
                        .to_string()
                        .into(),
                ));
        }
        self.state.embed(self.root)?;
        Ok(())
    }
}
