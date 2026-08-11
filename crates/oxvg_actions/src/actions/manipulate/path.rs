use lightningcss::values::shape::FillRule;
use oxvg_ast::{
    element::Element,
    get_attribute, get_computed_style, is_element, set_attribute,
    style::{self, ComputedStyles},
};
use oxvg_collections::attribute::inheritable::Inheritable;
use oxvg_path::{algorithm::bool_ops::OverlayRule, geometry::Tolerance, paths::segment};

use crate::{Action, Actor, Error};

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
        for selection in selections {
            #[allow(clippy::cast_sign_loss)]
            let Some(node) = self.allocator.get(selection as usize) else {
                continue;
            };
            let Some(element) = node.element() else {
                continue;
            };
            if !is_element!(element, Path) {
                continue;
            }
            if element.has_child_nodes() {
                continue;
            }
            let Some(path) = get_attribute!(element, D) else {
                continue;
            };
            let segment_path = segment::Path::from_svg(&path, &Tolerance::default());
            drop(path);
            let computed_styles = ComputedStyles::default()
                .with_all(&element, &styles)
                .map_err(|err| Error::ComputedStylesError(err.to_string()))?;
            let evenodd = get_computed_style!(computed_styles, FillRule)
                .map(|fill_rule| match fill_rule {
                    (Inheritable::Defined(FillRule::Nonzero) | Inheritable::Inherited, _) => false,
                    (Inheritable::Defined(FillRule::Evenodd), _) => true,
                })
                .unwrap_or(false);
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

            if let Some(element) = previous_element {
                element.remove();
            }
            previous_element = Some(element);
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

        Ok(())
    }
}
